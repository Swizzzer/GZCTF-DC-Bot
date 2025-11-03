use anyhow::Result;
use serde::{Deserialize, Serialize};
use serenity::all::Context;
use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::fs;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{Duration, sleep};
use tokio_util::sync::CancellationToken;

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
  persist_lock: Arc<Mutex<()>>,
  shutdown_token: CancellationToken,
  retry_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl MessageQueue {
  pub fn new(persist_path: String, messenger: Arc<DiscordMessenger>) -> Self {
    Self {
      queue: Arc::new(RwLock::new(VecDeque::new())),
      persist_path,
      messenger,
      persist_lock: Arc::new(Mutex::new(())),
      shutdown_token: CancellationToken::new(),
      retry_handle: Arc::new(Mutex::new(None)),
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
    let persist_lock = Arc::clone(&self.persist_lock);
    let shutdown_token = self.shutdown_token.clone();

    let handle = tokio::spawn(async move {
      log::info("Message queue retry loop started.");

      loop {
        tokio::select! {
          _ = shutdown_token.cancelled() => {
            log::info("Retry loop received shutdown signal, exiting...");
            break;
          }
          _ = sleep(Duration::from_secs(1)) => {
          }
        }

        // use read lock
        let items_to_retry: Vec<MessageItem> = {
          let queue_guard = queue.read().await;
          queue_guard
            .iter()
            .filter(|item| item.can_retry())
            .cloned()
            .collect()
        };
        // lock released

        if items_to_retry.is_empty() {
          continue;
        }

        let mut send_results = Vec::new();
        for item in items_to_retry {
          let embed = create_embed(
            &item.notice,
            item.notice_type.clone(),
            item.match_name.as_deref(),
            item.match_id,
            &item.base_url,
          );

          let result = messenger.send_embed(&ctx, embed).await;
          send_results.push((item.id.clone(), result));
        }

        // use write lock
        let mut to_persist = Vec::new();
        let mut remove_persist_succ = Vec::new();
        let mut remove_retry_succ = Vec::new();

        {
          let mut queue_guard = queue.write().await;

          for (msg_id, result) in send_results {
            if let Some(item) = queue_guard.iter_mut().find(|i| i.id == msg_id) {
              match result {
                Ok(_) => {
                  log::success(format!("Retry succeeded for message: {}", item.id));
                  remove_retry_succ.push(item.id.clone());
                }
                Err(e) => {
                  log::error(format!("Retry failed for message {}: {}", item.id, e));

                  if item.should_persist() {
                    log::info(format!(
                      "Message {} exceeded max retries. Persisting to disk.",
                      item.id
                    ));
                    to_persist.push(item.clone());
                    remove_persist_succ.push(item.id.clone());
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
          }

          queue_guard.retain(|item| !remove_retry_succ.contains(&item.id));
        }
        // lock released

        if !to_persist.is_empty() {
          match Self::append_to_disk(&persist_lock, &persist_path, &to_persist).await {
            Ok(_) => {
              // can be removed only if persisted successfully
              let mut queue_guard = queue.write().await;
              queue_guard.retain(|item| !remove_persist_succ.contains(&item.id));
              log::info(format!(
                "Removed {} persisted messages from queue.",
                remove_persist_succ.len()
              ));
            }
            Err(e) => {
              log::error(format!("Failed to persist messages to disk: {}", e));
              log::info("Messages will remain in queue for retry.");
            }
          }
        }
      }

      log::info("Retry loop finished.");
    });

    let mut retry_handle = self.retry_handle.lock().await;
    *retry_handle = Some(handle);
  }

  pub async fn shutdown(&self) -> Result<()> {
    log::info("Shutting down message queue...");

    self.shutdown_token.cancel();

    let handle = {
      let mut retry_handle = self.retry_handle.lock().await;
      retry_handle.take()
    };

    if let Some(h) = handle {
      log::info("Waiting for retry loop to finish...");
      if let Err(e) = h.await {
        log::error(format!("Error waiting for retry loop: {}", e));
      }
    }

    let queue_guard = self.queue.read().await;
    let remaining_items: Vec<MessageItem> = queue_guard.iter().cloned().collect();
    drop(queue_guard);

    if remaining_items.is_empty() {
      log::info("No pending messages to save.");
      return Ok(());
    }

    Self::append_to_disk(&self.persist_lock, &self.persist_path, &remaining_items).await?;
    log::success(format!(
      "Saved {} pending messages before shutdown.",
      remaining_items.len()
    ));

    Ok(())
  }

  async fn append_to_disk(
    persist_lock: &Mutex<()>,
    persist_path: &str,
    items: &[MessageItem],
  ) -> Result<()> {
    if items.is_empty() {
      return Ok(());
    }

    let _guard = persist_lock.lock().await;

    let path = Path::new(persist_path);

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
