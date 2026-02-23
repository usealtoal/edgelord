use std::path::Path;

use crate::adapter::inbound::cli::{operator, output};
use crate::error::Result;

/// Validate configuration file without starting the bot.
pub fn execute_config<P: AsRef<Path>>(config_path: P) -> Result<()> {
    let path = config_path.as_ref();
    let config_toml = operator::read_config_toml(path)?;
    let report = operator::operator().check_config(&config_toml)?;

    output::section("Configuration Check");
    output::field("Config", path.display());
    output::success("Configuration file is valid");

    output::section("Summary");
    output::field("Exchange", report.exchange);
    output::field("Environment", report.environment);
    output::field("Chain ID", report.chain_id);
    output::field("Strategies", format!("{:?}", report.enabled_strategies));
    output::field("Dry run", report.dry_run);

    if report.wallet_configured {
        output::success("Wallet credentials detected");
    } else {
        output::warning("Wallet credentials not configured (set WALLET_PRIVATE_KEY for trading)");
    }

    if report.telegram_enabled {
        if report.telegram_token_present && report.telegram_chat_present {
            output::success("Telegram integration configured");
        } else {
            output::warning("Telegram enabled but environment variables are missing");
            if !report.telegram_token_present {
                output::field("Missing", "TELEGRAM_BOT_TOKEN");
            }
            if !report.telegram_chat_present {
                output::field("Missing", "TELEGRAM_CHAT_ID");
            }
        }
    } else {
        output::field("Telegram", "disabled");
    }

    output::success("Configuration check complete");

    Ok(())
}
