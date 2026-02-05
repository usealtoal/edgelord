//! Handler for the `config` command group.

use std::fs;
use std::path::Path;

use rust_decimal::prelude::ToPrimitive;

use crate::app::{Config, Environment};
use crate::error::Result;

/// Default config template with documentation.
const CONFIG_TEMPLATE: &str = include_str!("../../config.toml.example");

/// Execute `config init`.
pub fn execute_init(path: &Path, force: bool) -> Result<()> {
    if path.exists() && !force {
        eprintln!("Config file already exists: {}", path.display());
        eprintln!("Use --force to overwrite.");
        std::process::exit(1);
    }

    fs::write(path, CONFIG_TEMPLATE)?;
    println!("Created config file: {}", path.display());
    println!();
    println!("Next steps:");
    println!("  1. Edit {} with your settings", path.display());
    println!("  2. Set WALLET_PRIVATE_KEY environment variable");
    println!("  3. Run: edgelord check config {}", path.display());
    println!("  4. Run: edgelord run -c {}", path.display());
    Ok(())
}

/// Execute `config show`.
pub fn execute_show(path: &Path) -> Result<()> {
    let config = Config::load(path)?;

    println!();
    println!("═══════════════════════════════════════════════════════════");
    println!("  Effective Configuration");
    println!("═══════════════════════════════════════════════════════════");
    println!();

    // General
    println!("  General");
    println!("  ─────────────────────────────────────────────────────────");
    println!("    Profile:       {:?}", config.profile);
    println!("    Dry Run:       {}", config.dry_run);
    println!();

    // Exchange
    let network = config.network();
    println!("  Exchange");
    println!("  ─────────────────────────────────────────────────────────");
    println!("    Type:          {:?}", config.exchange);
    println!("    Environment:   {:?}", network.environment);
    println!("    Chain ID:      {}", network.chain_id);
    println!("    WS URL:        {}", network.ws_url);
    println!("    API URL:       {}", network.api_url);
    println!();

    // Strategies
    println!("  Strategies");
    println!("  ─────────────────────────────────────────────────────────");
    for name in &config.strategies.enabled {
        println!("    • {name}");
    }
    if config.strategies.enabled.is_empty() {
        println!("    (none enabled)");
    }
    println!();

    // Risk
    println!("  Risk Management");
    println!("  ─────────────────────────────────────────────────────────");
    println!(
        "    Max Position:  ${}",
        config.risk.max_position_per_market
    );
    println!("    Max Exposure:  ${}", config.risk.max_total_exposure);
    println!("    Min Profit:    ${}", config.risk.min_profit_threshold);
    println!("    Max Slippage:  {:.1}%", config.risk.max_slippage.to_f64().unwrap_or(0.0) * 100.0);
    println!();

    // Wallet
    println!("  Wallet");
    println!("  ─────────────────────────────────────────────────────────");
    if config.wallet.private_key.is_some() {
        println!("    Private Key:   ✓ (from WALLET_PRIVATE_KEY)");
    } else {
        println!("    Private Key:   ✗ (not set)");
    }
    println!();

    // Notifications
    println!("  Notifications");
    println!("  ─────────────────────────────────────────────────────────");
    println!("    Telegram:      {}", if config.telegram.enabled { "✓ enabled" } else { "✗ disabled" });
    println!();

    // LLM
    println!("  LLM (Relation Inference)");
    println!("  ─────────────────────────────────────────────────────────");
    println!("    Provider:      {:?}", config.llm.provider);
    println!("    Inference:     {}", if config.inference.enabled { "✓ enabled" } else { "✗ disabled" });
    if config.inference.enabled {
        println!("    Min Confidence: {:.0}%", config.inference.min_confidence * 100.0);
        println!("    TTL:           {}s", config.inference.ttl_seconds);
    }
    println!();

    // Cluster Detection
    println!("  Cluster Detection");
    println!("  ─────────────────────────────────────────────────────────");
    println!("    Enabled:       {}", if config.cluster_detection.enabled { "✓" } else { "✗" });
    if config.cluster_detection.enabled {
        println!("    Debounce:      {}ms", config.cluster_detection.debounce_ms);
        println!("    Min Gap:       {:.1}%", config.cluster_detection.min_gap.to_f64().unwrap_or(0.0) * 100.0);
    }
    println!();

    Ok(())
}

/// Execute `config validate`.
pub fn execute_validate(path: &Path) -> Result<()> {
    println!("Validating: {}", path.display());
    println!();

    // Try to load
    match Config::load(path) {
        Ok(config) => {
            println!("✓ Config file is valid");
            println!();

            // Additional checks
            let mut warnings = Vec::new();

            if config.wallet.private_key.is_none() {
                warnings.push("WALLET_PRIVATE_KEY not set (required for trading)");
            }

            if config.strategies.enabled.is_empty() {
                warnings.push("No strategies enabled");
            }

            let network = config.network();
            if network.environment == Environment::Mainnet && config.dry_run {
                warnings.push("Mainnet configured but dry_run is enabled");
            }

            if config.inference.enabled {
                // Check if API key is available for the configured provider
                let has_api_key = match config.llm.provider {
                    crate::app::LlmProvider::Anthropic => {
                        std::env::var("ANTHROPIC_API_KEY").is_ok()
                    }
                    crate::app::LlmProvider::OpenAi => std::env::var("OPENAI_API_KEY").is_ok(),
                };
                if !has_api_key {
                    warnings.push("Inference enabled but LLM API key not set");
                }
            }

            if !warnings.is_empty() {
                println!("Warnings:");
                for w in warnings {
                    println!("  ⚠ {w}");
                }
                println!();
            }

            println!("Run 'edgelord config show -c {}' to see resolved values", path.display());
        }
        Err(e) => {
            println!("✗ Config file is invalid");
            println!();
            println!("Error: {e}");
            std::process::exit(1);
        }
    }

    Ok(())
}
