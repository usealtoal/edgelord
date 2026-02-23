use std::io::{self, Write};
use std::path::Path;

use rust_decimal::Decimal;

use crate::adapter::inbound::cli::{operator, output};
use crate::error::{ExecutionError, Result};
use crate::port::inbound::operator::wallet::ApprovalOutcome;

/// Approve token spending for trading.
///
/// # Arguments
///
/// * `config_path` - Path to the configuration file
/// * `amount` - Amount to approve in token units (e.g., dollars for USDC)
/// * `skip_confirm` - If true, skip the confirmation prompt
pub async fn execute_approve(
    config_path: &Path,
    amount: Decimal,
    skip_confirm: bool,
) -> Result<()> {
    let service = operator::operator();
    let config_toml = operator::read_config_toml(config_path)?;

    output::section("Wallet Approval");

    let pb = output::spinner("Fetching allowance...");
    let status = match service.wallet_status(&config_toml).await {
        Ok(status) => {
            output::spinner_success(&pb, "Fetched allowance");
            status
        }
        Err(e) => {
            output::spinner_fail(&pb, "Failed to fetch allowance");
            return Err(e);
        }
    };
    output::field("Exchange", status.exchange);
    output::field("Wallet", status.wallet_address);
    output::field("Token", status.token);
    output::field("Allowance", format!("${}", status.allowance));
    output::field("Spender", status.spender);
    output::field("Requested", format!("${amount}"));

    if !status.needs_approval && status.allowance >= amount {
        output::success(&format!(
            "Approval already satisfied (requested ${amount}, current ${})",
            status.allowance
        ));
        return Ok(());
    }

    if !skip_confirm {
        print!("Proceed with approval? [y/N] ");
        io::stdout().flush().ok();

        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();

        if !input.trim().eq_ignore_ascii_case("y") {
            output::warning("Approval cancelled by user");
            return Ok(());
        }
    }

    let pb = output::spinner("Submitting transaction...");
    let outcome = match service.wallet_approve(&config_toml, amount).await {
        Ok(outcome) => outcome,
        Err(e) => {
            output::spinner_fail(&pb, "Transaction failed");
            return Err(e);
        }
    };

    match outcome {
        ApprovalOutcome::Approved { tx_hash, amount } => {
            output::spinner_success(&pb, "Transaction confirmed");
            output::success("Approval successful");
            output::field("Amount", format!("${amount}"));
            output::field("Transaction", tx_hash);
        }
        ApprovalOutcome::AlreadyApproved { current_allowance } => {
            output::spinner_success(&pb, "Transaction confirmed");
            output::success(&format!(
                "Allowance already sufficient (current ${current_allowance})"
            ));
        }
        ApprovalOutcome::Failed { reason } => {
            output::spinner_fail(&pb, "Transaction failed");
            output::error(&format!("Approval failed: {reason}"));
            return Err(ExecutionError::OrderRejected(reason).into());
        }
    }

    Ok(())
}
