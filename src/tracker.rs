use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct NoticeTracker {
    counts: HashMap<String, usize>,
}

impl NoticeTracker {
    pub fn new() -> Self {
        Self {
            counts: HashMap::new(),
        }
    }

    pub fn get_count(&self, match_id: u32, notice_type: &str) -> usize {
        let key = format!("{}:{}", match_id, notice_type);
        *self.counts.get(&key).unwrap_or(&0)
    }

    pub fn set_count(&mut self, match_id: u32, notice_type: &str, count: usize) {
        let key = format!("{}:{}", match_id, notice_type);
        self.counts.insert(key, count);
    }
}
