use std::path::Path;

use crate::app::Config;
use crate::error::{Error, Result};

/// Test Telegram notification by sending a test message.
pub async fn execute_telegram<P: AsRef<Path>>(config_path: P) -> Result<()> {
    let config = Config::load(config_path)?;

    let token = std::env::var("TELEGRAM_BOT_TOKEN").map_err(|_| {
        crate::error::ConfigError::MissingField {
            field: "TELEGRAM_BOT_TOKEN environment variable",
        }
    })?;

    let chat_id = std::env::var("TELEGRAM_CHAT_ID").map_err(|_| {
        crate::error::ConfigError::MissingField {
            field: "TELEGRAM_CHAT_ID environment variable",
        }
    })?;

    println!("Sending test message to Telegram...");
    let masked_token = if token.len() >= 15 {
        format!("{}...{}", &token[..10], &token[token.len() - 5..])
    } else {
        format!("{}...", &token[..token.len().min(10)])
    };
    println!("  Bot token: {masked_token}");
    println!("  Chat ID: {chat_id}");
    println!();

    // Build the message
    let message = format!(
        "🧪 *Edgelord Test Message*\n\n\
        Configuration validated\\!\n\n\
        Environment: `{}`\n\
        Strategies: `{:?}`\n\
        Dry\\-run: `{}`",
        config.network().environment,
        config.strategies.enabled,
        config.dry_run
    );

    // Send via Telegram API
    let client = reqwest::Client::new();
    let url = format!("https://api.telegram.org/bot{token}/sendMessage");

    let response = client
        .post(&url)
        .json(&serde_json::json!({
            "chat_id": chat_id,
            "text": message,
            "parse_mode": "MarkdownV2"
        }))
        .send()
        .await
        .map_err(|e| Error::Connection(e.to_string()))?;

    if response.status().is_success() {
        println!("✓ Test message sent successfully!");
        println!();
        println!("Check your Telegram for the message.");
    } else {
        let status = response.status();
        let body: String = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(Error::Connection(format!(
            "failed to send telegram message: {status} {body}"
        )));
    }

    Ok(())
}
