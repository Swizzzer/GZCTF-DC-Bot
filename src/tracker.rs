use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct NoticeTracker {
    // 每种类型公告的最新时间戳：match_id:notice_type -> max_timestamp
    max_timestamps: HashMap<String, u64>,
}

impl NoticeTracker {
    pub fn new() -> Self {
        Self {
            max_timestamps: HashMap::new(),
        }
    }

    pub fn get_max_timestamp(&self, match_id: u32, notice_type: &str) -> u64 {
        let key = format!("{}:{}", match_id, notice_type);
        *self.max_timestamps.get(&key).unwrap_or(&0)
    }

    // 更新该类型公告的时间戳
    pub fn update_max_timestamp(&mut self, match_id: u32, notice_type: &str, timestamp: u64) {
        let key = format!("{}:{}", match_id, notice_type);
        let current_max = self.max_timestamps.entry(key).or_insert(0);
        if timestamp > *current_max {
            *current_max = timestamp;
        }
    }

    pub fn set_max_timestamp(&mut self, match_id: u32, notice_type: &str, timestamp: u64) {
        let key = format!("{}:{}", match_id, notice_type);
        self.max_timestamps.insert(key, timestamp);
    }
}
