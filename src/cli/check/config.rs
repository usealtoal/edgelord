use std::path::Path;

use crate::app::Config;
use crate::cli::output;
use crate::error::Result;

/// Validate configuration file without starting the bot.
pub fn execute_config<P: AsRef<Path>>(config_path: P) -> Result<()> {
    let path = config_path.as_ref();
    output::section("Configuration Check");
    output::key_value("Config", path.display());

    // Try to load and validate
    match Config::load(path) {
        Ok(config) => {
            output::ok("Configuration file is valid");

            output::section("Summary");
            output::key_value("Exchange", format!("{:?}", config.exchange));
            output::key_value("Environment", config.network().environment);
            output::key_value("Chain ID", config.network().chain_id);
            output::key_value("Strategies", format!("{:?}", config.strategies.enabled));
            output::key_value("Dry run", config.dry_run);

            // Check wallet
            if config.wallet.private_key.is_some() {
                output::ok("Wallet credentials detected");
            } else {
                output::warn(
                    "Wallet credentials not configured (set WALLET_PRIVATE_KEY for trading)",
                );
            }

            // Check telegram
            let telegram_token = std::env::var("TELEGRAM_BOT_TOKEN").ok();
            let telegram_chat = std::env::var("TELEGRAM_CHAT_ID").ok();

            if config.telegram.enabled {
                if telegram_token.is_some() && telegram_chat.is_some() {
                    output::ok("Telegram integration configured");
                } else {
                    output::warn("Telegram enabled but environment variables are missing");
                    if telegram_token.is_none() {
                        output::key_value("Missing", "TELEGRAM_BOT_TOKEN");
                    }
                    if telegram_chat.is_none() {
                        output::key_value("Missing", "TELEGRAM_CHAT_ID");
                    }
                }
            } else {
                output::key_value("Telegram", "disabled");
            }

            output::ok("Configuration check complete");
        }
        Err(e) => {
            return Err(e);
        }
    }

    Ok(())
}
