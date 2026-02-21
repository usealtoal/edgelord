use std::io::{self, Write};
use std::path::Path;

use rust_decimal::Decimal;

use crate::cli::output;
use crate::cli::wallet::{SweepOutcome, WalletService};
use crate::error::{ConfigError, Result};
use crate::runtime::{Config, Exchange};

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
    output::field("From", from_address);
    output::field("To", to);
    output::field("Asset", asset.to_uppercase());
    output::field("Network", network.to_lowercase());
    output::field("Balance", format!("${balance}"));

    if balance <= Decimal::ZERO {
        output::warning("No balance available to sweep");
        return Ok(());
    }

    if !skip_confirm {
        print!("Proceed with sweep? [y/N] ");
        io::stdout().flush().ok();

        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();

        if !input.trim().eq_ignore_ascii_case("y") {
            output::warning("Sweep cancelled by user");
            return Ok(());
        }
    }

    let pb = output::spinner("Submitting transaction");

    let outcome = match WalletService::sweep_usdc(&config, to).await {
        Ok(outcome) => outcome,
        Err(e) => {
            output::spinner_fail(&pb, "Submitting transaction");
            return Err(e);
        }
    };

    match outcome {
        SweepOutcome::NoBalance { .. } => {
            output::spinner_success(&pb, "Submitting transaction");
            output::warning("No balance available to sweep");
        }
        SweepOutcome::Transferred { tx_hash, amount } => {
            output::spinner_success(&pb, "Submitting transaction");
            output::success("Sweep transaction submitted");
            output::field("Amount", format!("${amount}"));
            output::field("Transaction", tx_hash);
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
