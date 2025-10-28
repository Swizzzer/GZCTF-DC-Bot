use anyhow::Result;
use serde::{Deserialize, Serialize};
use serenity::all::Context;
use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use tokio::time::{Duration, sleep};

use crate::discord::DiscordMessenger;
use crate::gzctf::create_embed;
use crate::log;
use crate::models::{Notice, NoticeType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageItem {
  pub id: String,
  pub notice: Notice,
  pub notice_type: NoticeType,
  pub match_name: Option<String>,
  pub match_id: u32,
  pub base_url: String,
  pub retry_count: u8,
  pub next_retry_at: u64,
}

impl MessageItem {
  pub fn new(
    id: String,
    notice: Notice,
    notice_type: NoticeType,
    match_name: Option<String>,
    match_id: u32,
    base_url: String,
  ) -> Self {
    Self {
      id,
      notice,
      notice_type,
      match_name,
      match_id,
      base_url,
      retry_count: 0,
      next_retry_at: Self::current_timestamp(),
    }
  }

  fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_secs()
  }

  // delay: 2**(retry_count+1)s
  pub fn calc_delay(&self) -> u64 {
    1u64 << (self.retry_count + 1)
  }

  pub fn increment_retry(&mut self) {
    self.retry_count += 1;
    let delay = self.calc_delay();
    self.next_retry_at = Self::current_timestamp() + delay;
  }

  pub fn can_retry(&self) -> bool {
    Self::current_timestamp() >= self.next_retry_at
  }

  pub fn should_persist(&self) -> bool {
    self.retry_count >= 4
  }
}

pub struct MessageQueue {
  queue: Arc<RwLock<VecDeque<MessageItem>>>,
  persist_path: String,
  messenger: Arc<DiscordMessenger>,
}

impl MessageQueue {
  pub fn new(persist_path: String, messenger: Arc<DiscordMessenger>) -> Self {
    Self {
      queue: Arc::new(RwLock::new(VecDeque::new())),
      persist_path,
      messenger,
    }
  }

  pub async fn load_from_disk(&self) -> Result<()> {
    let path = Path::new(&self.persist_path);

    if !path.exists() {
      log::info("No persisted messages found.");
      return Ok(());
    }

    let content = fs::read_to_string(path).await?;
    let items: Vec<MessageItem> = serde_json::from_str(&content)?;

    let mut queue = self.queue.write().await;
    for item in items {
      queue.push_back(item);
    }

    log::success(format!(
      "Loaded {} persisted messages from disk.",
      queue.len()
    ));

    drop(queue);
    fs::remove_file(path).await?;
    log::info("Cleared persist file after loading messages.");

    Ok(())
  }

  pub async fn enqueue(&self, message: MessageItem) {
    let mut queue = self.queue.write().await;
    queue.push_back(message.clone());
    log::info(format!(
      "Enqueued message: {} (retry_count={})",
      message.id, message.retry_count
    ));
  }

  pub async fn retrying(&self, ctx: Arc<Context>) {
    let queue = Arc::clone(&self.queue);
    let messenger = Arc::clone(&self.messenger);
    let persist_path = self.persist_path.clone();

    tokio::spawn(async move {
      log::info("Message queue retry loop started.");

      loop {
        sleep(Duration::from_secs(1)).await;

        let mut queue_guard = queue.write().await;
        let mut to_remove = Vec::new();
        let mut to_persist = Vec::new();

        for (idx, item) in queue_guard.iter_mut().enumerate() {
          if !item.can_retry() {
            continue;
          }

          let embed = create_embed(
            &item.notice,
            item.notice_type.clone(),
            item.match_name.as_deref(),
            item.match_id,
            &item.base_url,
          );

          match messenger.send_embed(&ctx, embed).await {
            Ok(_) => {
              log::success(format!("Retry succeeded for message: {}", item.id));
              to_remove.push(idx);
            }
            Err(e) => {
              log::error(format!("Retry failed for message {}: {}", item.id, e));

              if item.should_persist() {
                log::info(format!(
                  "Message {} exceeded max retries. Persisting to disk.",
                  item.id
                ));
                to_persist.push(item.clone());
                to_remove.push(idx);
              } else {
                item.increment_retry();
                let delay = item.calc_delay();
                log::info(format!(
                  "Message {} will retry in {}s (retry_count={})",
                  item.id, delay, item.retry_count
                ));
              }
            }
          }
        }

        for &idx in to_remove.iter().rev() {
          queue_guard.remove(idx);
        }

        drop(queue_guard);

        if !to_persist.is_empty() {
          if let Err(e) = Self::append_to_disk(&persist_path, &to_persist).await {
            log::error(format!("Failed to persist messages to disk: {}", e));
          }
        }
      }
    });
  }

  pub async fn shutdown(&self) -> Result<()> {
    log::info("Shutting down message queue...");

    let queue_guard = self.queue.read().await;
    let remaining_items: Vec<MessageItem> = queue_guard.iter().cloned().collect();
    drop(queue_guard);

    if remaining_items.is_empty() {
      log::info("No pending messages to save.");
      return Ok(());
    }

    Self::append_to_disk(&self.persist_path, &remaining_items).await?;
    log::success(format!(
      "Saved {} pending messages before shutdown.",
      remaining_items.len()
    ));

    Ok(())
  }

  async fn append_to_disk(persist_path: &str, items: &[MessageItem]) -> Result<()> {
    let path = Path::new(persist_path);

    if items.is_empty() {
      return Ok(());
    }

    let mut existing_items: Vec<MessageItem> = if path.exists() {
      let content = fs::read_to_string(path).await?;
      serde_json::from_str(&content).unwrap_or_default()
    } else {
      Vec::new()
    };

    existing_items.extend_from_slice(items);

    let json = serde_json::to_string_pretty(&existing_items)?;
    fs::write(path, json).await?;

    log::info(format!(
      "Appended {} messages to persist file.",
      items.len()
    ));
    Ok(())
  }
}
