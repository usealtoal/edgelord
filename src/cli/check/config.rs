use std::path::Path;

use crate::app::Config;
use crate::error::Result;

/// Validate configuration file without starting the bot.
pub fn execute_config<P: AsRef<Path>>(config_path: P) -> Result<()> {
    let path = config_path.as_ref();
    println!("Checking configuration: {}", path.display());
    println!();

    // Try to load and validate
    match Config::load(path) {
        Ok(config) => {
            println!("✓ Configuration file is valid");
            println!();
            println!("Summary:");
            println!("  Exchange: {:?}", config.exchange);
            println!("  Environment: {}", config.network().environment);
            println!("  Chain ID: {}", config.network().chain_id);
            println!("  Strategies: {:?}", config.strategies.enabled);
            println!("  Dry-run: {}", config.dry_run);
            println!();

            // Check wallet
            if config.wallet.private_key.is_some() {
                println!("✓ Wallet private key found (from WALLET_PRIVATE_KEY env var)");
            } else {
                println!("⚠ No wallet private key configured");
                println!("  Set WALLET_PRIVATE_KEY environment variable for trading");
            }

            // Check telegram
            let telegram_token = std::env::var("TELEGRAM_BOT_TOKEN").ok();
            let telegram_chat = std::env::var("TELEGRAM_CHAT_ID").ok();

            if config.telegram.enabled {
                if telegram_token.is_some() && telegram_chat.is_some() {
                    println!("✓ Telegram configured and enabled");
                } else {
                    println!("⚠ Telegram enabled but missing environment variables:");
                    if telegram_token.is_none() {
                        println!("    - TELEGRAM_BOT_TOKEN");
                    }
                    if telegram_chat.is_none() {
                        println!("    - TELEGRAM_CHAT_ID");
                    }
                }
            } else {
                println!("  Telegram: disabled");
            }

            println!();
            println!("Configuration is ready to use.");
        }
        Err(e) => {
            return Err(e);
        }
    }

    Ok(())
}
