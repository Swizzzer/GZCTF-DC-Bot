use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::Config;
use crate::polling::PollingService;
use crate::tracker::NoticeTracker;

pub struct BotHandler {
    pub config: Arc<Config>,
    pub tracker: Arc<RwLock<NoticeTracker>>,
}

#[async_trait]
impl EventHandler for BotHandler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("[+] {} is connected and ready!", ready.user.name);

        let config = Arc::clone(&self.config);
        let tracker = Arc::clone(&self.tracker);
        let ctx = Arc::new(ctx);

        tokio::spawn(async move {
            match PollingService::new(config, tracker) {
                Ok(service) => {
                    let service = Arc::new(service);
                    if let Err(e) = service.start_polling(ctx).await {
                        eprintln!("[-] Error in polling loop: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("[-] Failed to create polling service: {}", e);
                }
            }
        });
    }

    async fn message(&self, _ctx: Context, msg: Message) {
        if msg.content == "!ping" {
            println!("📨 Received ping from {}", msg.author.name);
        }
    }
}
