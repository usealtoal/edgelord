//! Configuration and connection validation commands.

use std::path::Path;

use crate::app::Config;
use crate::error::Result;

/// Validate configuration file without starting the bot.
pub fn execute_config<P: AsRef<Path>>(config_path: P) {
    let path = config_path.as_ref();
    println!("Checking configuration: {}", path.display());
    println!();

    // Check file exists
    if !path.exists() {
        eprintln!("Error: Configuration file not found: {}", path.display());
        eprintln!();
        eprintln!("Create one by copying the example:");
        eprintln!("  cp config.toml.example config.toml");
        std::process::exit(1);
    }

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
            eprintln!("✗ Configuration error: {e}");
            std::process::exit(1);
        }
    }
}

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
        .map_err(|e| crate::error::Error::Connection(e.to_string()))?;

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
        eprintln!("✗ Failed to send message: {status}");
        eprintln!("  {body}");
        std::process::exit(1);
    }

    Ok(())
}

/// Test WebSocket connection to the exchange.
pub async fn execute_connection<P: AsRef<Path>>(config_path: P) -> Result<()> {
    let config = Config::load(config_path)?;
    let network = config.network();

    println!("Testing connection to {:?}...", config.exchange);
    println!("  WebSocket: {}", network.ws_url);
    println!("  API: {}", network.api_url);
    println!("  Environment: {}", network.environment);
    println!();

    // Test REST API connectivity
    print!("Testing REST API... ");
    let client = reqwest::Client::new();
    let api_url = format!("{}/markets", network.api_url);

    match client.get(&api_url).send().await {
        Ok(response) if response.status().is_success() => {
            println!("✓ OK");
        }
        Ok(response) => {
            println!("✗ HTTP {}", response.status());
            eprintln!("API returned non-success status");
            std::process::exit(1);
        }
        Err(e) => {
            println!("✗ Failed");
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }

    // Test WebSocket connectivity
    print!("Testing WebSocket... ");
    match tokio_tungstenite::connect_async(&network.ws_url).await {
        Ok((_, _)) => {
            println!("✓ OK");
        }
        Err(e) => {
            println!("✗ Failed");
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }

    println!();
    println!("All connection tests passed.");

    Ok(())
}
