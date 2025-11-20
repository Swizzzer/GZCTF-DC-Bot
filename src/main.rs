mod config;
mod discord;
mod gzctf;
mod handler;
mod log;
mod models;
mod polling;
mod queue;
mod tracker;

use anyhow::Result;
use clap::Parser;
use config::Config;
use discord::DiscordMessenger;
use handler::BotHandler;
use queue::MessageQueue;
use serenity::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, timeout};
use tracker::NoticeTracker;

#[derive(Parser, Debug)]
#[command(name = "dc-bot")]
#[command(version, about, long_about = None)]
struct Cli {
  #[arg(short, long, default_value = "config.toml")]
  config: String,
}

#[tokio::main]
async fn main() -> Result<()> {
  let cli = Cli::parse();

  let config = Config::from_file(&cli.config).unwrap_or_else(|e| {
    log::error(format!(
      "Failed to read config file '{}': {}",
      cli.config, e
    ));
    std::process::exit(1);
  });

  print_config_info(&config);

  let config = Arc::new(config);
  let tracker = Arc::new(RwLock::new(NoticeTracker::new()));

  let messenger = Arc::new(DiscordMessenger::new(config.discord.channel_id));
  let persist_path = "failed_messages.json".to_string();
  let message_queue = Arc::new(MessageQueue::new(persist_path, messenger));

  if let Err(e) = message_queue.load_from_disk().await {
    log::error(format!("Failed to load persisted messages: {}", e));
  }

  let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

  let handler = BotHandler {
    config: Arc::clone(&config),
    tracker: Arc::clone(&tracker),
    message_queue: Arc::clone(&message_queue),
  };

  let mut client = timeout(
    Duration::from_secs(10),
    Client::builder(&config.discord.token, intents).event_handler(handler),
  )
  .await
  .unwrap_or_else(|_| {
    log::error("Timed out creating Discord client");
    std::process::exit(1);
  })
  .unwrap_or_else(|e| {
    log::error(format!("Failed to create Discord client: {}", e));
    std::process::exit(1);
  });

  log::success("Starting Discord bot...\n");

  let client_task = tokio::spawn(async move {
    if let Err(why) = client.start().await {
      log::error(format!("Client error: {:?}", why));
    }
  });

  tokio::select! {
    _ = tokio::signal::ctrl_c() => {
      log::info("\nReceived Ctrl+C, shutting down...");
    }
    _ = client_task => {
      log::info("Client task finished.");
    }
  }

  if let Err(e) = message_queue.shutdown().await {
    log::error(format!("Failed to save messages on shutdown: {}", e));
  }

  Ok(())
}

fn print_config_info(config: &Config) {
  log::info("Configuration loaded:");
  log::info(format!("   GZCTF URL: {}", config.gzctf.url));
  log::info(format!("   Channel ID: {}", config.discord.channel_id));
  log::info(format!("   Poll interval: {}s", config.gzctf.poll_interval));

  let matches = config.get_matches();
  log::info(format!("   Matches to monitor: {}", matches.len()));

  matches.iter().for_each(|match_config| {
    let msg = match &match_config.name {
      Some(name) => format!("      - ID {} ({})", match_config.id, name),
      None => format!("      - ID {}", match_config.id),
    };
    log::info(msg);
  });

  println!();
}
