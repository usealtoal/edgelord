use std::io::{self, Write};
use std::path::Path;

use rust_decimal::Decimal;

use crate::adapter::inbound::cli::{operator, output};
use crate::error::Result;
use crate::port::inbound::operator::wallet::SweepOutcome;

/// Sweep the full USDC balance to the provided address.
pub async fn execute_sweep(
    config_path: &Path,
    to: &str,
    asset: &str,
    network: &str,
    skip_confirm: bool,
) -> Result<()> {
    let service = operator::operator();
    let config_toml = operator::read_config_toml(config_path)?;
    let balance = service.wallet_balance(&config_toml).await?;
    let from_address = service.wallet_address(&config_toml)?;

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

    let outcome = match service.wallet_sweep(&config_toml, to, asset, network).await {
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
