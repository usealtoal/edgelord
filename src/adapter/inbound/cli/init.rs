//! Interactive setup wizard.

use std::path::PathBuf;

use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};

use crate::adapter::inbound::cli::output;
use crate::error::{ConfigError, Result};

/// Default config template used by the setup wizard.
const CONFIG_TEMPLATE: &str = include_str!("../../../../config.toml.example");

/// Run the interactive setup wizard.
pub fn execute(path: PathBuf, force: bool) -> Result<()> {
    if output::is_json() {
        return Err(ConfigError::InvalidValue {
            field: "json",
            reason: "`edgelord init` is interactive; use `edgelord config init` for scripted setup"
                .to_string(),
        }
        .into());
    }

    output::header(env!("CARGO_PKG_VERSION"));
    output::section("Interactive Setup");
    output::note("Welcome to edgelord.");
    output::note("Let's set up your configuration.");

    let theme = ColorfulTheme::default();

    // Network selection.
    let networks = &["Testnet (recommended for first run)", "Mainnet"];
    let network = Select::with_theme(&theme)
        .with_prompt("Network")
        .items(networks)
        .default(0)
        .interact()?;

    let environment = if network == 0 { "testnet" } else { "mainnet" };

    // Strategy selection.
    output::section("Strategies");

    let single_condition = Confirm::with_theme(&theme)
        .with_prompt("Enable single_condition (YES + NO < $1)")
        .default(true)
        .interact()?;

    let rebalancing = Confirm::with_theme(&theme)
        .with_prompt("Enable market_rebalancing (multi-outcome)")
        .default(true)
        .interact()?;

    let combinatorial = Confirm::with_theme(&theme)
        .with_prompt("Enable combinatorial (requires LLM API key)")
        .default(false)
        .interact()?;

    let mut enabled_strategies = Vec::new();
    if single_condition {
        enabled_strategies.push("single_condition");
    }
    if rebalancing {
        enabled_strategies.push("market_rebalancing");
    }
    if combinatorial {
        enabled_strategies.push("combinatorial");
    }

    if enabled_strategies.is_empty() {
        output::warning("No strategies selected.");
        let proceed = Confirm::with_theme(&theme)
            .with_prompt("Continue with no enabled strategies?")
            .default(false)
            .interact()?;
        if !proceed {
            output::warning("Setup aborted.");
            return Ok(());
        }
    }

    // Risk settings.
    output::section("Risk limits");

    let max_exposure: f64 = Input::with_theme(&theme)
        .with_prompt("Maximum total exposure ($)")
        .default(500.0)
        .interact()?;

    let max_position: f64 = Input::with_theme(&theme)
        .with_prompt("Maximum per-market position ($)")
        .default(100.0)
        .interact()?;

    // Generate config.
    let config = generate_config(
        environment,
        &enabled_strategies,
        max_exposure,
        max_position,
        combinatorial,
    )?;

    // Check if file exists.
    if path.exists() && !force {
        let overwrite = Confirm::with_theme(&theme)
            .with_prompt(format!("{} already exists. Overwrite?", path.display()))
            .default(false)
            .interact()?;
        if !overwrite {
            output::warning("Setup aborted.");
            return Ok(());
        }
    }

    // Write config.
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, config)?;

    output::success(&format!("Created {}", path.display()));
    output::section("Next Steps");
    output::note("1. Configure wallet credentials:");
    output::note(&format!(
        "   - env key: {}",
        output::highlight("WALLET_PRIVATE_KEY=<hex-without-0x>")
    ));
    output::note(&format!(
        "   - or keystore: {}",
        output::highlight("edgelord provision polymarket --wallet generate")
    ));
    output::note(&format!(
        "2. Validate: {}",
        output::highlight(format!("edgelord check config -c {}", path.display()))
    ));
    output::note(&format!(
        "3. Health: {}",
        output::highlight(format!("edgelord check health -c {}", path.display()))
    ));
    output::note(&format!(
        "4. Readiness: {}",
        output::highlight(format!("edgelord check live -c {}", path.display()))
    ));
    output::note(&format!(
        "5. Start: {}",
        output::highlight(format!("edgelord run -c {}", path.display()))
    ));

    Ok(())
}

fn generate_config(
    environment: &str,
    strategies: &[&str],
    max_exposure: f64,
    max_position: f64,
    enable_combinatorial: bool,
) -> Result<String> {
    let mut config: toml::Value = toml::from_str(CONFIG_TEMPLATE).map_err(ConfigError::Parse)?;
    let table = config.as_table_mut().ok_or_else(|| {
        ConfigError::Other("config template root must be a TOML table".to_string())
    })?;

    table.insert(
        "dry_run".to_string(),
        toml::Value::Boolean(environment == "testnet"),
    );

    if let Some(exchange_config) = table
        .get_mut("exchange_config")
        .and_then(toml::Value::as_table_mut)
    {
        exchange_config.insert(
            "environment".to_string(),
            toml::Value::String(environment.to_string()),
        );
        exchange_config.insert(
            "chain_id".to_string(),
            toml::Value::Integer(if environment == "mainnet" { 137 } else { 80002 }),
        );
    }

    if let Some(strategies_table) = table
        .get_mut("strategies")
        .and_then(toml::Value::as_table_mut)
    {
        strategies_table.insert(
            "enabled".to_string(),
            toml::Value::Array(
                strategies
                    .iter()
                    .map(|name| toml::Value::String((*name).to_string()))
                    .collect(),
            ),
        );
    }

    if let Some(combinatorial_table) = table
        .get_mut("strategies")
        .and_then(toml::Value::as_table_mut)
        .and_then(|strategies_table| {
            strategies_table
                .get_mut("combinatorial")
                .and_then(toml::Value::as_table_mut)
        })
    {
        combinatorial_table.insert(
            "enabled".to_string(),
            toml::Value::Boolean(enable_combinatorial),
        );
    }

    if let Some(risk_table) = table.get_mut("risk").and_then(toml::Value::as_table_mut) {
        risk_table.insert(
            "max_total_exposure".to_string(),
            toml::Value::Float(max_exposure),
        );
        risk_table.insert(
            "max_position_per_market".to_string(),
            toml::Value::Float(max_position),
        );
    }

    toml::to_string_pretty(&config).map_err(|error| ConfigError::Other(error.to_string()).into())
}
