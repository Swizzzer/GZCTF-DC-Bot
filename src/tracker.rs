use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::fs;

use crate::log;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct NoticeTracker {
  // 每种类型公告的最新时间戳：match_id:notice_type -> max_timestamp
  max_timestamps: HashMap<String, u64>,
  #[serde(skip)]
  persist_path: Option<String>,
}

impl NoticeTracker {
  #[allow(dead_code)]
  pub fn new() -> Self {
    Self {
      max_timestamps: HashMap::new(),
      persist_path: None,
    }
  }

  pub fn with_persist_path(persist_path: String) -> Self {
    Self {
      max_timestamps: HashMap::new(),
      persist_path: Some(persist_path),
    }
  }

  pub async fn load_from_disk(persist_path: &str) -> Result<Self> {
    if !fs::try_exists(persist_path).await.unwrap_or(false) {
      log::info("No persisted tracker found, starting fresh.");
      return Ok(Self::with_persist_path(persist_path.to_string()));
    }

    let content = fs::read_to_string(persist_path).await?;
    let mut tracker: NoticeTracker = serde_json::from_str(&content)?;
    tracker.persist_path = Some(persist_path.to_string());

    log::success(format!(
      "Loaded {} tracked timestamps from disk.",
      tracker.max_timestamps.len()
    ));

    Ok(tracker)
  }

  pub async fn save_to_disk(&self) -> Result<()> {
    let Some(ref persist_path) = self.persist_path else {
      return Ok(());
    };

    let json = serde_json::to_string_pretty(&self)?;

    // Atomic write: write to temp file first, then rename
    let tmp_path = format!("{}.tmp", persist_path);
    fs::write(&tmp_path, &json).await?;
    fs::rename(&tmp_path, persist_path).await?;

    Ok(())
  }

  pub fn get_timestamp(&self, match_id: u32, notice_type: &str) -> u64 {
    let key = format!("{}:{}", match_id, notice_type);
    *self.max_timestamps.get(&key).unwrap_or(&0)
  }

  pub fn update_timestamp(&mut self, match_id: u32, notice_type: &str, timestamp: u64) {
    let key = format!("{}:{}", match_id, notice_type);
    let current_max = self.max_timestamps.entry(key).or_insert(0);
    if timestamp > *current_max {
      *current_max = timestamp;
    }
  }

  pub fn set_timestamp(&mut self, match_id: u32, notice_type: &str, timestamp: u64) {
    let key = format!("{}:{}", match_id, notice_type);
    self.max_timestamps.insert(key, timestamp);
  }
}
