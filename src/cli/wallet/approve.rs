use std::io::{self, Write};
use std::path::Path;

use rust_decimal::Decimal;

use crate::app::{ApprovalOutcome, Config, WalletService};
use crate::cli::output;
use crate::error::{ExecutionError, Result};

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

    output::progress("Fetching current allowance");
    let status = match WalletService::get_approval_status(&config).await {
        Ok(status) => {
            output::progress_done(true);
            status
        }
        Err(e) => {
            output::progress_done(false);
            return Err(e);
        }
    };
    output::key_value("Exchange", status.exchange);
    output::key_value("Wallet", status.wallet_address);
    output::key_value("Token", status.token);
    output::key_value("Allowance", format!("${}", status.allowance));
    output::key_value("Spender", status.spender);
    output::key_value("Requested", format!("${amount}"));

    if !status.needs_approval && status.allowance >= amount {
        output::ok(&format!(
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
            output::warn("Approval cancelled by user");
            return Ok(());
        }
    }

    output::progress("Submitting transaction");
    let outcome = match WalletService::approve(&config, amount).await {
        Ok(outcome) => outcome,
        Err(e) => {
            output::progress_done(false);
            return Err(e);
        }
    };

    match outcome {
        ApprovalOutcome::Approved { tx_hash, amount } => {
            output::progress_done(true);
            output::ok("Approval successful");
            output::key_value("Amount", format!("${amount}"));
            output::key_value("Transaction", tx_hash);
        }
        ApprovalOutcome::AlreadyApproved { current_allowance } => {
            output::progress_done(true);
            output::ok(&format!(
                "Allowance already sufficient (current ${current_allowance})"
            ));
        }
        ApprovalOutcome::Failed { reason } => {
            output::progress_done(false);
            output::error(&format!("Approval failed: {reason}"));
            return Err(ExecutionError::OrderRejected(reason).into());
        }
    }

    Ok(())
}
