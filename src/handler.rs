use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::Config;
use crate::log;
use crate::polling::PollingService;
use crate::queue::MessageQueue;
use crate::tracker::NoticeTracker;

pub struct BotHandler {
  pub config: Arc<Config>,
  pub tracker: Arc<RwLock<NoticeTracker>>,
  pub message_queue: Arc<MessageQueue>,
}

#[async_trait]
impl EventHandler for BotHandler {
  async fn ready(&self, ctx: Context, ready: Ready) {
    log::success(format!("{} is connected and ready!", ready.user.name));

    let config = Arc::clone(&self.config);
    let tracker = Arc::clone(&self.tracker);
    let message_queue = Arc::clone(&self.message_queue);
    let ctx = Arc::new(ctx);

    message_queue.retrying(Arc::clone(&ctx)).await;

    tokio::spawn(async move {
      match PollingService::new(config, tracker, message_queue).map(Arc::new) {
        Ok(service) => {
          if let Err(e) = service.start_polling(ctx).await {
            log::error(format!("Polling service error: {}", e));
          }
        }
        Err(e) => log::error(format!("Polling service error: {}", e)),
      }
    });
  }

  async fn message(&self, _ctx: Context, msg: Message) {
    if msg.content == "!ping" {
      log::info(format!("Received ping from {}", msg.author.name));
    }
  }
}
