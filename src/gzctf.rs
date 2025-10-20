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

pub fn format_time(timestamp_ms: u64) -> String {
    let timestamp_secs = (timestamp_ms / 1000) as i64;

    if let Some(dt) = DateTime::from_timestamp(timestamp_secs, 0) {
        let beijing_time = dt.with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap());
        beijing_time.format("%Y-%m-%d %H:%M:%S").to_string()
    } else {
        format!("{}", timestamp_ms)
    }
}

pub fn format_message(
    notice: &Notice,
    notice_type: NoticeType,
    match_name: Option<&str>,
    match_id: u32,
    base_url: &str,
) -> String {
    let title = notice_type.get_title();
    let formatted_time = format_time(notice.time);

    // Create match name as a hyperlink
    let match_link = if let Some(name) = match_name {
        let game_url = format!("{}/games/{}", base_url, match_id);
        format!("[{}]({})", name, game_url)
    } else {
        String::new()
    };

    let content = match notice_type {
        NoticeType::Normal => notice.values.get(0).cloned().unwrap_or_default(),
        NoticeType::NewChallenge | NoticeType::NewHint => {
            notice.values.get(0).cloned().unwrap_or_default()
        }
        NoticeType::FirstBlood => {
            if notice.values.len() >= 2 {
                format!("恭喜{}获得{}的一血", notice.values[0], notice.values[1])
            } else {
                notice.values.join(" - ")
            }
        }
        NoticeType::SecondBlood => {
            if notice.values.len() >= 2 {
                format!("恭喜{}获得{}的二血", notice.values[0], notice.values[1])
            } else {
                notice.values.join(" - ")
            }
        }
        NoticeType::ThirdBlood => {
            if notice.values.len() >= 2 {
                format!("恭喜{}获得{}的三血", notice.values[0], notice.values[1])
            } else {
                notice.values.join(" - ")
            }
        }
    };
    // Message header
    let header = if !match_link.is_empty() {
        format!("📢 {} {}", match_link, title)
    } else {
        format!("📢 {}", title)
    };

    match notice_type {
        NoticeType::Normal => {
            format!(
                "{}\n内容：{}\n时间：{}",
                header, content, formatted_time
            )
        }
        NoticeType::NewChallenge | NoticeType::NewHint => {
            format!("{}\n{}\n时间：{}", header, content, formatted_time)
        }
        _ => {
            format!("{}\n{}\n时间：{}", header, content, formatted_time)
        }
    }
}
