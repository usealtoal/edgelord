use std::path::Path;

use crate::app::Config;
use crate::cli::output;
use crate::error::Result;

/// Validate configuration file without starting the bot.
pub fn execute_config<P: AsRef<Path>>(config_path: P) -> Result<()> {
    let path = config_path.as_ref();
    output::section("Configuration Check");
    output::field("Config", path.display());

    // Try to load and validate
    match Config::load(path) {
        Ok(config) => {
            output::success("Configuration file is valid");

            output::section("Summary");
            output::field("Exchange", format!("{:?}", config.exchange));
            output::field("Environment", config.network().environment);
            output::field("Chain ID", config.network().chain_id);
            output::field("Strategies", format!("{:?}", config.strategies.enabled));
            output::field("Dry run", config.dry_run);

            // Check wallet
            if config.wallet.private_key.is_some() {
                output::success("Wallet credentials detected");
            } else {
                output::warning(
                    "Wallet credentials not configured (set WALLET_PRIVATE_KEY for trading)",
                );
            }

            // Check telegram
            let telegram_token = std::env::var("TELEGRAM_BOT_TOKEN").ok();
            let telegram_chat = std::env::var("TELEGRAM_CHAT_ID").ok();

            if config.telegram.enabled {
                if telegram_token.is_some() && telegram_chat.is_some() {
                    output::success("Telegram integration configured");
                } else {
                    output::warning("Telegram enabled but environment variables are missing");
                    if telegram_token.is_none() {
                        output::field("Missing", "TELEGRAM_BOT_TOKEN");
                    }
                    if telegram_chat.is_none() {
                        output::field("Missing", "TELEGRAM_CHAT_ID");
                    }
                }
            } else {
                output::field("Telegram", "disabled");
            }

            output::success("Configuration check complete");
        }
        Err(e) => {
            return Err(e);
        }
    }

    Ok(())
}
