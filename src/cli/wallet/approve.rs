use std::io::{self, Write};
use std::path::Path;

use rust_decimal::Decimal;

use crate::cli::output;
use crate::cli::wallet::{ApprovalOutcome, WalletService};
use crate::error::{ExecutionError, Result};
use crate::infrastructure::Config;

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
    let config = Config::load(config_path)?;

    output::section("Wallet Approval");

    let pb = output::spinner("Fetching current allowance");
    let status = match WalletService::get_approval_status(&config).await {
        Ok(status) => {
            output::spinner_success(&pb, "Fetching current allowance");
            status
        }
        Err(e) => {
            output::spinner_fail(&pb, "Fetching current allowance");
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

    let pb = output::spinner("Submitting transaction");
    let outcome = match WalletService::approve(&config, amount).await {
        Ok(outcome) => outcome,
        Err(e) => {
            output::spinner_fail(&pb, "Submitting transaction");
            return Err(e);
        }
    };

    match outcome {
        ApprovalOutcome::Approved { tx_hash, amount } => {
            output::spinner_success(&pb, "Submitting transaction");
            output::success("Approval successful");
            output::field("Amount", format!("${amount}"));
            output::field("Transaction", tx_hash);
        }
        ApprovalOutcome::AlreadyApproved { current_allowance } => {
            output::spinner_success(&pb, "Submitting transaction");
            output::success(&format!(
                "Allowance already sufficient (current ${current_allowance})"
            ));
        }
        ApprovalOutcome::Failed { reason } => {
            output::spinner_fail(&pb, "Submitting transaction");
            output::error(&format!("Approval failed: {reason}"));
            return Err(ExecutionError::OrderRejected(reason).into());
        }
    }

    Ok(())
}
