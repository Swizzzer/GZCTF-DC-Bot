mod config;
mod discord;
mod gzctf;
mod handler;
mod log;
mod models;
mod polling;
mod tracker;

use anyhow::Result;
use clap::Parser;
use serenity::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;

use config::Config;
use handler::BotHandler;
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

    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let handler = BotHandler {
        config: Arc::clone(&config),
        tracker: Arc::clone(&tracker),
    };

    let mut client = Client::builder(&config.discord.token, intents)
        .event_handler(handler)
        .await
        .expect("Failed to create Discord client");

    log::success("Starting Discord bot...\n");

    if let Err(why) = client.start().await {
        log::error(format!("Client error: {:?}", why));
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
