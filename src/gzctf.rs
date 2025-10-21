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

        self.client
            .get(&api_url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .map_err(Into::into)
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

    DateTime::from_timestamp(timestamp_secs, 0)
        .map(|dt| {
            let beijing_tz = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
            dt.with_timezone(&beijing_tz)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        })
        .unwrap_or_else(|| timestamp_ms.to_string())
}

// 截断文本以避免队伍名过长影响观感
fn trunc_text(text: &str, max_len: usize) -> String {
    let char_count = text.chars().count();

    (char_count > max_len)
        .then(|| format!("{}…", text.chars().take(max_len - 1).collect::<String>()))
        .unwrap_or_else(|| text.to_string())
}

pub fn create_embed(
    notice: &Notice,
    notice_type: NoticeType,
    match_name: Option<&str>,
    match_id: u32,
    base_url: &str,
) -> CreateEmbed {
    let game_url = format!("{}/games/{}", base_url, match_id);

    let mut embed = CreateEmbed::new()
        .title(notice_type.get_title())
        .color(get_notice_color(&notice_type))
        .footer(CreateEmbedFooter::new(format_time(notice.time)));

    if let Some(name) = match_name {
        embed = embed.description(format!("**赛事:** [{}]({})", name, game_url));
    }

    embed = add_notice_fields(embed, &notice_type, &notice.values);

    embed
}

fn get_notice_color(notice_type: &NoticeType) -> Colour {
    match notice_type {
        NoticeType::Normal => Colour::from_rgb(59, 130, 246), // Blue
        NoticeType::NewChallenge => Colour::from_rgb(34, 197, 94), // Green
        NoticeType::NewHint => Colour::from_rgb(234, 179, 8), // Yellow
        NoticeType::FirstBlood => Colour::from_rgb(239, 68, 68), // Red
        NoticeType::SecondBlood => Colour::from_rgb(249, 115, 22), // Orange
        NoticeType::ThirdBlood => Colour::from_rgb(168, 85, 247), // Purple
    }
}

fn add_notice_fields(
    embed: CreateEmbed,
    notice_type: &NoticeType,
    values: &[String],
) -> CreateEmbed {
    match notice_type {
        NoticeType::Normal => embed.field(
            "公告内容",
            values.first().cloned().unwrap_or_default(),
            false,
        ),
        NoticeType::NewChallenge | NoticeType::NewHint => {
            embed.field("题目", values.first().cloned().unwrap_or_default(), false)
        }
        NoticeType::FirstBlood | NoticeType::SecondBlood | NoticeType::ThirdBlood => embed
            .field("队伍", trunc_text(&values[0], 30), false)
            .field("题目", &values[1], false),
    }
}
