//! Handler for the `config` command group.

use std::fs;
use std::path::Path;

use rust_decimal::prelude::ToPrimitive;

use crate::adapter::inbound::cli::{operator, output};
use crate::error::{ConfigError, Result};

/// Default config template with documentation.
const CONFIG_TEMPLATE: &str = include_str!("../../../../config.toml.example");

/// Execute `config init`.
pub fn execute_init(path: &Path, force: bool) -> Result<()> {
    if path.exists() && !force {
        return Err(ConfigError::InvalidValue {
            field: "config",
            reason: "file already exists (use --force to overwrite)".to_string(),
        }
        .into());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, CONFIG_TEMPLATE)?;
    output::section("Config Initialized");
    output::success("Created configuration file");
    output::field("Path", path.display());
    output::section("Next Steps");
    output::note(&format!("1. Edit {} with your settings", path.display()));
    output::note("2. Set WALLET_PRIVATE_KEY environment variable");
    output::note(&format!("3. Run: edgelord check config {}", path.display()));
    output::note(&format!("4. Run: edgelord run -c {}", path.display()));
    Ok(())
}

/// Execute `config show`.
pub fn execute_show(path: &Path) -> Result<()> {
    let config_toml = operator::read_config_toml(path)?;
    let config = operator::operator().show_config(&config_toml)?;

    output::section("Effective Configuration");
    output::field("Profile", config.profile);
    output::field("Dry run", config.dry_run);

    output::section("Exchange");
    output::field("Type", config.exchange);
    output::field("Environment", config.environment);
    output::field("Chain ID", config.chain_id);
    output::field("WebSocket", config.ws_url);
    output::field("API", config.api_url);

    output::section("Strategies");
    if config.enabled_strategies.is_empty() {
        output::note("(none enabled)");
    } else {
        for name in &config.enabled_strategies {
            output::note(&format!("- {name}"));
        }
    }

    output::section("Risk");
    output::field(
        "Max position",
        format!("${}", config.risk.max_position_per_market),
    );
    output::field(
        "Max exposure",
        format!("${}", config.risk.max_total_exposure),
    );
    output::field(
        "Min profit",
        format!("${}", config.risk.min_profit_threshold),
    );
    output::field(
        "Max slippage",
        format!(
            "{:.1}%",
            config.risk.max_slippage.to_f64().unwrap_or(0.0) * 100.0
        ),
    );

    output::section("Wallet");
    if config.wallet_private_key_loaded {
        output::success("Private key loaded from WALLET_PRIVATE_KEY");
    } else {
        output::warning("Private key not set");
    }

    output::section("Notifications");
    output::field(
        "Telegram",
        if config.telegram_enabled {
            "enabled"
        } else {
            "disabled"
        },
    );

    output::section("LLM Inference");
    output::field("Provider", config.llm_provider);
    output::field(
        "Enabled",
        if config.inference.enabled {
            "yes"
        } else {
            "no"
        },
    );
    if config.inference.enabled {
        output::field(
            "Min confidence",
            format!("{:.0}%", config.inference.min_confidence * 100.0),
        );
        output::field("TTL", format!("{}s", config.inference.ttl_seconds));
    }

    output::section("Cluster Detection");
    output::field(
        "Enabled",
        if config.cluster_detection.enabled {
            "yes"
        } else {
            "no"
        },
    );
    if config.cluster_detection.enabled {
        output::field(
            "Debounce",
            format!("{}ms", config.cluster_detection.debounce_ms),
        );
        output::field(
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
    output::field("Path", path.display());
    let config_toml = operator::read_config_toml(path)?;
    let validation = operator::operator().validate_config(&config_toml)?;
    output::success("Config file is valid");

    if !validation.warnings.is_empty() {
        output::section("Warnings");
        for warning in &validation.warnings {
            output::warning(warning);
        }
    }

    output::field(
        "Next",
        format!("edgelord config show -c {}", path.display()),
    );

    Ok(())
}
