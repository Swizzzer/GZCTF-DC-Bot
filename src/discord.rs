use anyhow::Result;
use serenity::model::id::ChannelId;
use serenity::prelude::*;

pub struct DiscordMessenger {
    channel_id: u64,
}

impl DiscordMessenger {
    pub fn new(channel_id: u64) -> Self {
        Self { channel_id }
    }

    pub async fn send_message(&self, ctx: &Context, content: &str) -> Result<()> {
        let channel = ChannelId::new(self.channel_id);
        channel.say(&ctx.http, content).await?;
        println!("Sent message to channel {}", self.channel_id);
        Ok(())
    }
}
