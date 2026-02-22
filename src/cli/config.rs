//! Handler for the `config` command group.

use std::fs;
use std::path::Path;

use rust_decimal::prelude::ToPrimitive;

use crate::cli::output;
use crate::error::{ConfigError, Result};
use crate::infrastructure::{Config, Environment};

/// Default config template with documentation.
const CONFIG_TEMPLATE: &str = include_str!("../../config.toml.example");

/// Execute `config init`.
pub fn execute_init(path: &Path, force: bool) -> Result<()> {
    if path.exists() && !force {
        return Err(ConfigError::InvalidValue {
            field: "config",
            reason: "file already exists (use --force to overwrite)".to_string(),
        }
        .into());
    }

    fs::write(path, CONFIG_TEMPLATE)?;
    output::section("Config Initialized");
    output::ok("Created configuration file");
    output::key_value("Path", path.display());
    output::section("Next Steps");
    output::note(&format!("1. Edit {} with your settings", path.display()));
    output::note("2. Set WALLET_PRIVATE_KEY environment variable");
    output::note(&format!("3. Run: edgelord check config {}", path.display()));
    output::note(&format!("4. Run: edgelord run -c {}", path.display()));
    Ok(())
}

/// Execute `config show`.
pub fn execute_show(path: &Path) -> Result<()> {
    let config = Config::load(path)?;
    let network = config.network();

    output::section("Effective Configuration");
    output::key_value("Profile", format!("{:?}", config.profile));
    output::key_value("Dry run", config.dry_run);

    output::section("Exchange");
    output::key_value("Type", format!("{:?}", config.exchange));
    output::key_value("Environment", network.environment);
    output::key_value("Chain ID", network.chain_id);
    output::key_value("WebSocket", &network.ws_url);
    output::key_value("API", &network.api_url);

    output::section("Strategies");
    if config.strategies.enabled.is_empty() {
        output::note("(none enabled)");
    } else {
        for name in &config.strategies.enabled {
            output::note(&format!("- {name}"));
        }
    }

    output::section("Risk");
    output::key_value(
        "Max position",
        format!("${}", config.risk.max_position_per_market),
    );
    output::key_value(
        "Max exposure",
        format!("${}", config.risk.max_total_exposure),
    );
    output::key_value(
        "Min profit",
        format!("${}", config.risk.min_profit_threshold),
    );
    output::key_value(
        "Max slippage",
        format!(
            "{:.1}%",
            config.risk.max_slippage.to_f64().unwrap_or(0.0) * 100.0
        ),
    );

    output::section("Wallet");
    if config.wallet.private_key.is_some() {
        output::ok("Private key loaded from WALLET_PRIVATE_KEY");
    } else {
        output::warn("Private key not set");
    }

    output::section("Notifications");
    output::key_value(
        "Telegram",
        if config.telegram.enabled {
            "enabled"
        } else {
            "disabled"
        },
    );

    output::section("LLM Inference");
    output::key_value("Provider", format!("{:?}", config.llm.provider));
    output::key_value(
        "Enabled",
        if config.inference.enabled {
            "yes"
        } else {
            "no"
        },
    );
    if config.inference.enabled {
        output::key_value(
            "Min confidence",
            format!("{:.0}%", config.inference.min_confidence * 100.0),
        );
        output::key_value("TTL", format!("{}s", config.inference.ttl_seconds));
    }

    output::section("Cluster Detection");
    output::key_value(
        "Enabled",
        if config.cluster_detection.enabled {
            "yes"
        } else {
            "no"
        },
    );
    if config.cluster_detection.enabled {
        output::key_value(
            "Debounce",
            format!("{}ms", config.cluster_detection.debounce_ms),
        );
        output::key_value(
            "Min gap",
            format!(
                "{:.1}%",
                config.cluster_detection.min_gap.to_f64().unwrap_or(0.0) * 100.0
            ),
        );
    }

    Ok(())
}

/// Execute `config validate`.
pub fn execute_validate(path: &Path) -> Result<()> {
    output::section("Config Validation");
    output::key_value("Path", path.display());

    // Try to load
    match Config::load(path) {
        Ok(config) => {
            output::ok("Config file is valid");

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
                    crate::infrastructure::LlmProvider::Anthropic => {
                        std::env::var("ANTHROPIC_API_KEY").is_ok()
                    }
                    crate::infrastructure::LlmProvider::OpenAi => std::env::var("OPENAI_API_KEY").is_ok(),
                };
                if !has_api_key {
                    warnings.push("Inference enabled but LLM API key not set");
                }
            }

            if !warnings.is_empty() {
                output::section("Warnings");
                for w in warnings {
                    output::warn(w);
                }
            }

            output::key_value(
                "Next",
                format!("edgelord config show -c {}", path.display()),
            );
        }
        Err(e) => {
            return Err(e);
        }
    }

    Ok(())
}
