use std::io::{self, Write};
use std::path::Path;

use rust_decimal::Decimal;

use crate::app::{Config, Exchange, SweepOutcome, WalletService};
use crate::cli::output;
use crate::error::{ConfigError, Result};

/// Sweep the full USDC balance to the provided address.
pub async fn execute_sweep(
    config_path: &Path,
    to: &str,
    asset: &str,
    network: &str,
    skip_confirm: bool,
) -> Result<()> {
    let config = Config::load(config_path)?;

    validate_sweep_inputs(config.exchange, asset, network)?;

    let balance = WalletService::usdc_balance(&config).await?;
    let from_address = WalletService::wallet_address(&config)?;

    output::section("Wallet Sweep");
    output::key_value("From", from_address);
    output::key_value("To", to);
    output::key_value("Asset", asset.to_uppercase());
    output::key_value("Network", network.to_lowercase());
    output::key_value("Balance", format!("${balance}"));

    if balance <= Decimal::ZERO {
        output::warn("No balance available to sweep");
        return Ok(());
    }

    if !skip_confirm {
        print!("Proceed with sweep? [y/N] ");
        io::stdout().flush().ok();

        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();

        if !input.trim().eq_ignore_ascii_case("y") {
            output::warn("Sweep cancelled by user");
            return Ok(());
        }
    }

    print!("Submitting transaction... ");
    io::stdout().flush().ok();

    match WalletService::sweep_usdc(&config, to).await? {
        SweepOutcome::NoBalance { .. } => {
            println!("ok");
            output::warn("No balance available to sweep");
        }
        SweepOutcome::Transferred { tx_hash, amount } => {
            println!("ok");
            output::ok("Sweep transaction submitted");
            output::key_value("Amount", format!("${amount}"));
            output::key_value("Transaction", tx_hash);
        }
    }

    Ok(())
}

fn validate_sweep_inputs(exchange: Exchange, asset: &str, network: &str) -> Result<()> {
    let asset_normalized = asset.trim().to_lowercase();
    let network_normalized = network.trim().to_lowercase();

    match exchange {
        Exchange::Polymarket => {
            if asset_normalized != "usdc" {
                return Err(ConfigError::InvalidValue {
                    field: "asset",
                    reason: "only usdc is supported for Polymarket sweeps".to_string(),
                }
                .into());
            }
            if network_normalized != "polygon" {
                return Err(ConfigError::InvalidValue {
                    field: "network",
                    reason: "only polygon is supported for Polymarket sweeps".to_string(),
                }
                .into());
            }
        }
    }

    Ok(())
}
