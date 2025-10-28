use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, sleep};

use crate::config::{Config, MatchConfig};
use crate::discord::DiscordMessenger;
use crate::gzctf::{GzctfClient, create_embed};
use crate::log;
use crate::models::{Notice, NoticeType};
use crate::queue::{MessageItem, MessageQueue};
use crate::tracker::NoticeTracker;
use serenity::prelude::Context;

pub struct PollingService {
  config: Arc<Config>,
  gzctf_client: GzctfClient,
  messenger: DiscordMessenger,
  tracker: Arc<RwLock<NoticeTracker>>,
  message_queue: Arc<MessageQueue>,
}

impl PollingService {
  pub fn new(
    config: Arc<Config>,
    tracker: Arc<RwLock<NoticeTracker>>,
    message_queue: Arc<MessageQueue>,
  ) -> Result<Self> {
    let gzctf_client = GzctfClient::new(config.gzctf.url.clone())?;
    let messenger = DiscordMessenger::new(config.discord.channel_id);

    Ok(Self {
      config,
      gzctf_client,
      messenger,
      tracker,
      message_queue,
    })
  }

  async fn init_counts(&self, matches: &[MatchConfig]) {
    let notice_types = NoticeType::all();

    for match_config in matches {
      let result = self.init_match(match_config, &notice_types).await;
      let match_name = match_config.name.as_deref().unwrap_or("未命名比赛");

      match result {
        Ok(_) => log::success(format!(
          "Initialized tracker for match {} ({})",
          match_config.id, match_name
        )),
        Err(e) => log::error(format!(
          "Failed to initialize tracker for match {}: {}",
          match_config.id, e
        )),
      }
    }
  }

  async fn init_match(
    &self,
    match_config: &MatchConfig,
    notice_types: &[NoticeType],
  ) -> Result<()> {
    let notices = self.gzctf_client.fetch_notices(match_config.id).await?;
    let mut tracker = self.tracker.write().await;

    notice_types.iter().for_each(|notice_type| {
      let filtered = GzctfClient::filter_by_type(&notices, notice_type.clone());
      let type_str = format!("{:?}", notice_type);

      filtered.iter().map(|n| n.time).max().map(|max_time| {
        tracker.set_timestamp(match_config.id, &type_str, max_time);
        log::info(format!(
          "   {:?}: latest timestamp = {}",
          notice_type, max_time
        ));
      });
    });

    Ok(())
  }

  async fn check_match(&self, ctx: &Context, match_config: &MatchConfig) -> Result<()> {
    let notice_types = NoticeType::all();
    let notices = self.gzctf_client.fetch_notices(match_config.id).await?;
    let mut tracker = self.tracker.write().await;

    for notice_type in &notice_types {
      self
        .handle_notices(ctx, match_config, notice_type, &notices, &mut tracker)
        .await;
    }
    Ok(())
  }

  async fn handle_notices(
    &self,
    ctx: &Context,
    match_config: &MatchConfig,
    notice_type: &NoticeType,
    notices: &[Notice],
    tracker: &mut tokio::sync::RwLockWriteGuard<'_, NoticeTracker>,
  ) {
    let type_str = format!("{:?}", notice_type);
    let filtered = GzctfClient::filter_by_type(notices, notice_type.clone());
    let last_timestamp = tracker.get_timestamp(match_config.id, &type_str);
    let new_notices = self.get_new_notices(&filtered, last_timestamp);
    if !new_notices.is_empty() {
      self.log_new_notice(match_config, notice_type, new_notices.len());
      self
        .broadcast(
          ctx,
          match_config,
          notice_type,
          new_notices,
          tracker,
          &type_str,
        )
        .await;
    }
  }

  fn get_new_notices<'a>(&self, notices: &'a [Notice], last_max: u64) -> Vec<&'a Notice> {
    let mut new_notices: Vec<_> = notices.iter().filter(|n| n.time > last_max).collect();
    new_notices.sort_by_key(|n| n.time);
    new_notices
  }

  async fn broadcast(
    &self,
    ctx: &Context,
    match_config: &MatchConfig,
    notice_type: &NoticeType,
    notices: Vec<&Notice>,
    tracker: &mut tokio::sync::RwLockWriteGuard<'_, NoticeTracker>,
    type_str: &str,
  ) {
    for notice in notices {
      self
        .broadcast_single(ctx, match_config, notice_type, notice)
        .await
        .unwrap_or_else(|e| log::error(format!("Failed to send embed message: {}", e)));

      tracker.update_timestamp(match_config.id, type_str, notice.time);
    }
  }

  async fn broadcast_single(
    &self,
    ctx: &Context,
    match_config: &MatchConfig,
    notice_type: &NoticeType,
    notice: &Notice,
  ) -> Result<()> {
    log::info(format!(
      "   Broadcasting notice ID {} (time: {}, type: {:?})",
      notice.id, notice.time, notice_type
    ));

    let embed = create_embed(
      notice,
      notice_type.clone(),
      match_config.name.as_deref(),
      match_config.id,
      &self.config.gzctf.url,
    );

    match self.messenger.send_embed(ctx, embed).await {
      Ok(_) => Ok(()),
      Err(e) => {
        log::error(format!(
          "Failed to send message: {}. Adding to retry queue.",
          e
        ));

        let message_id = format!("{}:{}:{}", match_config.id, notice.id, notice.time);
        let message_item = MessageItem::new(
          message_id,
          notice.clone(),
          notice_type.clone(),
          match_config.name.clone(),
          match_config.id,
          self.config.gzctf.url.clone(),
        );
        self.message_queue.enqueue(message_item).await;

        Err(e)
      }
    }
  }

  pub async fn start_polling(self: Arc<Self>, ctx: Arc<Context>) -> Result<()> {
    let matches = self.config.get_matches();

    if matches.is_empty() {
      log::error("No matches configured to monitor!");
      return Ok(());
    }

    self.log_match_info(&matches);
    self.init_counts(&matches).await;

    loop {
      sleep(Duration::from_secs(self.config.gzctf.poll_interval)).await;
      log::info("Polling for new notices...");
      self.poll_matches(&ctx, &matches).await;
    }
  }

  async fn poll_matches(&self, ctx: &Context, matches: &[MatchConfig]) {
    for match_config in matches {
      self
        .check_match(ctx, match_config)
        .await
        .unwrap_or_else(|e| {
          log::error(format!(
            "Failed to fetch notices for match {}: {}",
            match_config.id, e
          ))
        });
    }
  }
  fn log_match_info(&self, matches: &[MatchConfig]) {
    log::info(format!("Monitoring {} match(es)", matches.len()));

    matches.iter().for_each(|match_config| {
      let match_name = match_config.name.as_deref().unwrap_or("未命名比赛");
      log::info(format!(
        "   - Match ID {} ({})",
        match_config.id, match_name
      ));
    });
  }

  fn log_new_notice(&self, match_config: &MatchConfig, notice_type: &NoticeType, count: usize) {
    let match_name = match_config.name.as_deref().unwrap_or("未命名比赛");
    log::info(format!(
      "[Match {} - {}] Found {} new {:?} notice(s)",
      match_config.id, match_name, count, notice_type
    ));
  }
}
