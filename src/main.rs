mod config;
mod discord;
mod gzctf;
mod handler;
mod models;
mod polling;
mod tracker;

use anyhow::Result;
use serenity::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;

use config::Config;
use handler::BotHandler;
use tracker::NoticeTracker;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_file("config.toml")
        .expect("Failed to read config.toml. Please create it with your bot token and settings.");

    print_config_info(&config);

    let config = Arc::new(config);
    let tracker = Arc::new(RwLock::new(NoticeTracker::new()));

    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let handler = BotHandler {
        config: Arc::clone(&config),
        tracker: Arc::clone(&tracker),
    };

    let mut client = Client::builder(&config.discord.token, intents)
        .event_handler(handler)
        .await
        .expect("Failed to create Discord client");

    println!("[+] Starting Discord bot...\n");

    if let Err(why) = client.start().await {
        eprintln!("[-] Client error: {:?}", why);
    }

    Ok(())
}

fn print_config_info(config: &Config) {
    println!("📋 Configuration loaded:");
    println!("   GZCTF URL: {}", config.gzctf.url);
    println!("   Channel ID: {}", config.discord.channel_id);
    println!("   Poll interval: {}s", config.gzctf.poll_interval);

    let matches = config.get_matches();
    println!("   Matches to monitor: {}", matches.len());
    for match_config in &matches {
        if let Some(name) = &match_config.name {
            println!("      - ID {} ({})", match_config.id, name);
        } else {
            println!("      - ID {}", match_config.id);
        }
    }
    println!();
}
