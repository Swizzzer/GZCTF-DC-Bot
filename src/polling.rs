use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, sleep};

use crate::config::{Config, MatchConfig};
use crate::discord::DiscordMessenger;
use crate::gzctf::{GzctfClient, format_message};
use crate::models::NoticeType;
use crate::tracker::NoticeTracker;
use serenity::prelude::Context;

pub struct PollingService {
    config: Arc<Config>,
    gzctf_client: GzctfClient,
    messenger: DiscordMessenger,
    tracker: Arc<RwLock<NoticeTracker>>,
}

impl PollingService {
    pub fn new(config: Arc<Config>, tracker: Arc<RwLock<NoticeTracker>>) -> Result<Self> {
        let gzctf_client = GzctfClient::new(config.gzctf.url.clone())?;
        let messenger = DiscordMessenger::new(config.discord.channel_id);

        Ok(Self {
            config,
            gzctf_client,
            messenger,
            tracker,
        })
    }

    async fn initialize_counts(&self, matches: &[MatchConfig]) {
        let notice_types = NoticeType::all();

        for match_config in matches {
            match self.gzctf_client.fetch_notices(match_config.id).await {
                Ok(notices) => {
                    let mut tracker_write = self.tracker.write().await;
                    for notice_type in &notice_types {
                        let filtered = GzctfClient::filter_by_type(&notices, notice_type.clone());
                        let type_str = format!("{:?}", notice_type);
                        if let Some(max_time) = filtered.iter().map(|n| n.time).max() {
                            tracker_write.set_max_timestamp(match_config.id, &type_str, max_time);
                            println!("   📅 {:?}: latest timestamp = {}", notice_type, max_time);
                        }
                    }
                    drop(tracker_write);
                    let name_str = match_config.name.as_deref().unwrap_or("未命名比赛");
                    println!(
                        "[+] Initialized tracker for match {} ({})",
                        match_config.id, name_str
                    );
                }
                Err(e) => {
                    eprintln!(
                        "[-]  Failed to initialize tracker for match {}: {}",
                        match_config.id, e
                    );
                }
            }
        }
    }

    async fn check_match(&self, ctx: &Context, match_config: &MatchConfig) -> Result<()> {
        let notice_types = NoticeType::all();
        let notices = self.gzctf_client.fetch_notices(match_config.id).await?;
        let mut tracker_write = self.tracker.write().await;

        for notice_type in &notice_types {
            let type_str = format!("{:?}", notice_type);
            let filtered = GzctfClient::filter_by_type(&notices, notice_type.clone());
            let last_max_timestamp = tracker_write.get_max_timestamp(match_config.id, &type_str);

            let new_notices: Vec<_> = filtered
                .iter()
                .filter(|n| n.time > last_max_timestamp)
                .collect();

            if !new_notices.is_empty() {
                let name_str = match_config.name.as_deref().unwrap_or("未命名比赛");
                println!(
                    "🔭 [Match {} - {}] Found {} new {:?} notice(s)",
                    match_config.id,
                    name_str,
                    new_notices.len(),
                    notice_type
                );
                let mut sorted_notices = new_notices.clone();
                sorted_notices.sort_by_key(|n| n.time);

                for notice in &sorted_notices {
                    println!(
                        "   📤 Broadcasting notice ID {} (time: {}, type: {:?})",
                        notice.id, notice.time, notice_type
                    );
                    let message = format_message(
                        notice,
                        notice_type.clone(),
                        match_config.name.as_deref(),
                        match_config.id,
                        &self.config.gzctf.url,
                    );
                    if let Err(e) = self.messenger.send_message(ctx, &message).await {
                        eprintln!("[-] Failed to send message: {}", e);
                    }

                    // 更新最新时间戳
                    tracker_write.update_max_timestamp(match_config.id, &type_str, notice.time);
                }
            }
        }

        Ok(())
    }

    pub async fn start_polling(self: Arc<Self>, ctx: Arc<Context>) -> Result<()> {
        let matches = self.config.get_matches();

        if matches.is_empty() {
            eprintln!("[-] No matches configured to monitor!");
            return Ok(());
        }

        println!("[*] Monitoring {} match(es)", matches.len());
        for match_config in &matches {
            let name_str = match_config.name.as_deref().unwrap_or("未命名比赛");
            println!("   - Match ID {} ({})", match_config.id, name_str);
        }

        self.initialize_counts(&matches).await;

        loop {
            sleep(Duration::from_secs(self.config.gzctf.poll_interval)).await;

            println!("🔍 Polling for new notices...");

            for match_config in &matches {
                if let Err(e) = self.check_match(&ctx, match_config).await {
                    eprintln!(
                        "[-] Failed to fetch notices for match {}: {}",
                        match_config.id, e
                    );
                }
            }
        }
    }
}
