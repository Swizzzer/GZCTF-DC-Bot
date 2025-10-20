use std::collections::{HashMap, HashSet};

#[derive(Debug, Default)]
pub struct NoticeTracker {
    // 已播报的公告 ID：match_id:notice_type -> Set<notice_id>
    seen_ids: HashMap<String, HashSet<u64>>,
}

impl NoticeTracker {
    pub fn new() -> Self {
        Self {
            seen_ids: HashMap::new(),
        }
    }

    pub fn is_seen(&self, match_id: u32, notice_type: &str, notice_id: u64) -> bool {
        let key = format!("{}:{}", match_id, notice_type);
        self.seen_ids
            .get(&key)
            .map(|ids| ids.contains(&notice_id))
            .unwrap_or(false)
    }

    pub fn mark_seen(&mut self, match_id: u32, notice_type: &str, notice_id: u64) {
        let key = format!("{}:{}", match_id, notice_type);
        self.seen_ids
            .entry(key)
            .or_insert_with(HashSet::new)
            .insert(notice_id);
    }

    pub fn mark_all_seen(&mut self, match_id: u32, notice_type: &str, notice_ids: Vec<u64>) {
        let key = format!("{}:{}", match_id, notice_type);
        self.seen_ids.insert(key, notice_ids.into_iter().collect());
    }
}
