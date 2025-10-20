use anyhow::Result;
use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::model::id::ChannelId;
use serenity::prelude::*;

pub struct DiscordMessenger {
    channel_id: u64,
}

impl DiscordMessenger {
    pub fn new(channel_id: u64) -> Self {
        Self { channel_id }
    }

    pub async fn send_embed(&self, ctx: &Context, embed: CreateEmbed) -> Result<()> {
        let channel = ChannelId::new(self.channel_id);
        let builder = CreateMessage::new().embed(embed);
        channel.send_message(&ctx.http, builder).await?;
        println!("Sent embed message to channel {}", self.channel_id);
        Ok(())
    }
}
