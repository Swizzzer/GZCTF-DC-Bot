use anyhow::Result;
use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use tokio::time::{Duration, timeout};

use crate::log;

pub struct DiscordMessenger {
  channel_id: u64,
}

impl DiscordMessenger {
  pub fn new(channel_id: u64) -> Self {
    Self { channel_id }
  }

  pub async fn send_embed(&self, ctx: &Context, embed: CreateEmbed) -> Result<()> {
    let send_future =
      ChannelId::new(self.channel_id).send_message(&ctx.http, CreateMessage::new().embed(embed));

    match timeout(Duration::from_secs(10), send_future).await {
      Ok(Ok(_)) => {
        log::success(format!("Sent embed message to channel {}", self.channel_id));
        Ok(())
      }
      Ok(Err(e)) => {
        log::error(format!(
          "Failed to send message to channel {}: {}",
          self.channel_id, e
        ));
        Err(e.into())
      }
      Err(_) => {
        log::error(format!(
          "Timeout (10s) while sending message to channel {}",
          self.channel_id
        ));
        Err(anyhow::anyhow!("Message send timeout after 10 seconds"))
      }
    }
  }
}
