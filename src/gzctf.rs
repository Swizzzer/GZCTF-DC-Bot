use anyhow::Result;
use chrono::DateTime;

use crate::models::{Notice, NoticeType};

pub struct GzctfClient {
    base_url: String,
    client: reqwest::Client,
}

impl GzctfClient {
    pub fn new(base_url: String) -> Result<Self> {
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()?;

        Ok(Self { base_url, client })
    }

    pub async fn fetch_notices(&self, match_id: u32) -> Result<Vec<Notice>> {
        let api_url = format!("{}/api/game/{}/notices", self.base_url, match_id);

        let response = self.client.get(&api_url).send().await?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to fetch notices: HTTP {}", response.status());
        }

        let notices: Vec<Notice> = response.json().await?;
        Ok(notices)
    }

    pub fn filter_by_type(notices: &[Notice], notice_type: NoticeType) -> Vec<Notice> {
        notices
            .iter()
            .filter(|n| NoticeType::from_str(&n.notice_type) == Some(notice_type.clone()))
            .cloned()
            .collect()
    }
}

pub fn format_time(time_str: &str) -> String {
    if let Ok(dt) = DateTime::parse_from_rfc3339(time_str) {
        let beijing_time = dt.with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap());
        beijing_time.format("%Y-%m-%d %H:%M:%S").to_string()
    } else {
        time_str.to_string()
    }
}

pub fn format_message(
    notice: &Notice,
    notice_type: NoticeType,
    match_name: Option<&str>,
) -> String {
    let title = notice_type.get_title();
    let formatted_time = format_time(&notice.time);

    let prefix = if let Some(name) = match_name {
        format!("[{}] ", name)
    } else {
        String::new()
    };

    match notice_type {
        NoticeType::Normal => {
            format!(
                "{}{}\n内容：{}\n时间：{}",
                prefix, title, notice.content, formatted_time
            )
        }
        NoticeType::NewChallenge => {
            format!(
                "{}{}\n{}\n时间：{}",
                prefix, title, notice.content, formatted_time
            )
        }
        _ => {
            format!(
                "{}{}\n{}\n时间：{}",
                prefix, title, notice.content, formatted_time
            )
        }
    }
}
