use anyhow::Result;
use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::model::id::ChannelId;
use serenity::prelude::*;

use crate::log;

pub struct DiscordMessenger {
  channel_id: u64,
}

impl DiscordMessenger {
  pub fn new(channel_id: u64) -> Self {
    Self { channel_id }
  }

  pub async fn send_embed(&self, ctx: &Context, embed: CreateEmbed) -> Result<()> {
    ChannelId::new(self.channel_id)
      .send_message(&ctx.http, CreateMessage::new().embed(embed))
      .await
      .map(|_| log::success(format!("Sent embed message to channel {}", self.channel_id)))
      .map_err(Into::into)
  }
}
