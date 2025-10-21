use anyhow::Result;
use chrono::DateTime;
use serenity::builder::{CreateEmbed, CreateEmbedFooter};
use serenity::model::colour::Colour;

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

// 截断文本以避免队伍名过长影响观感
fn truncate_text(text: &str, max_len: usize) -> String {
    if text.chars().count() > max_len {
        let truncated: String = text.chars().take(max_len - 1).collect();
        format!("{}…", truncated)
    } else {
        text.to_string()
    }
}

pub fn create_embed(
    notice: &Notice,
    notice_type: NoticeType,
    match_name: Option<&str>,
    match_id: u32,
    base_url: &str,
) -> CreateEmbed {
    let title = notice_type.get_title();
    let formatted_time = format_time(notice.time);
    let game_url = format!("{}/games/{}", base_url, match_id);

    let color = match notice_type {
        NoticeType::Normal => Colour::from_rgb(59, 130, 246), // Blue
        NoticeType::NewChallenge => Colour::from_rgb(34, 197, 94), // Green
        NoticeType::NewHint => Colour::from_rgb(234, 179, 8), // Yellow
        NoticeType::FirstBlood => Colour::from_rgb(239, 68, 68), // Red
        NoticeType::SecondBlood => Colour::from_rgb(249, 115, 22), // Orange
        NoticeType::ThirdBlood => Colour::from_rgb(168, 85, 247), // Purple
    };

    let footer = CreateEmbedFooter::new(formatted_time);
    let mut embed = CreateEmbed::new().title(title).color(color).footer(footer);

    if let Some(name) = match_name {
        let match_info = format!("**赛事:** [{}]({})", name, game_url);
        embed = embed.description(&match_info);
    }

    match notice_type {
        NoticeType::Normal => {
            let content = notice.values.get(0).cloned().unwrap_or_default();
            embed = embed.field("公告内容", content, false);
        }
        NoticeType::NewChallenge | NoticeType::NewHint => {
            let content = notice.values.get(0).cloned().unwrap_or_default();
            embed = embed.field("题目", content, false);
        }
        NoticeType::FirstBlood | NoticeType::SecondBlood | NoticeType::ThirdBlood => {
            if notice.values.len() >= 2 {
                let team = &notice.values[0];
                let challenge = &notice.values[1];

                let team_display = truncate_text(team, 30);

                embed = embed
                    .field("队伍", team_display, false)
                    .field("题目", challenge, false);
            }
        }
    }

    embed
}
