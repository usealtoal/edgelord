//! Wallet operations CLI handlers.
//!
//! Provides handlers for wallet-related subcommands using the app layer
//! wallet facade. This keeps CLI code focused on user interaction while
//! delegating business logic to the app layer.

use std::io::{self, Write};
use std::path::Path;

use rust_decimal::Decimal;

use crate::app::{ApprovalOutcome, Config, WalletService};
use crate::error::{ExecutionError, Result};

/// Display current wallet approval status.
///
/// Shows the current token allowance and whether additional approval is needed.
pub async fn execute_status(config_path: &Path) -> Result<()> {
    let config = Config::load(config_path)?;

    println!();
    println!("Wallet Status");
    println!("\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}");

    print!("Fetching approval status... ");
    io::stdout().flush().ok();

    let status = WalletService::get_approval_status(&config).await?;

    println!("done");
    println!();
    println!("Exchange:    {}", status.exchange);
    println!("Wallet:      {}", status.wallet_address);
    println!("Token:       {}", status.token);
    println!("Allowance:   ${}", status.allowance);
    println!("Spender:     {}", status.spender);
    println!();

    if status.needs_approval {
        println!("Status:      \u{25cb} needs approval");
        println!();
        println!("Run 'edgelord wallet approve' to approve token spending.");
    } else {
        println!("Status:      \u{25cf} approved");
    }

    println!();
    Ok(())
}

/// Approve token spending for trading.
///
/// # Arguments
///
/// * `config_path` - Path to the configuration file
/// * `amount` - Amount to approve in token units (e.g., dollars for USDC)
/// * `skip_confirm` - If true, skip the confirmation prompt
pub async fn execute_approve(config_path: &Path, amount: Decimal, skip_confirm: bool) -> Result<()> {
    let config = Config::load(config_path)?;

    println!();
    println!("Token Approval");
    println!("\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}");

    // Get current status first
    print!("Fetching current allowance... ");
    io::stdout().flush().ok();

    let status = WalletService::get_approval_status(&config).await?;

    println!("done");
    println!();
    println!("Exchange:    {}", status.exchange);
    println!("Wallet:      {}", status.wallet_address);
    println!("Token:       {}", status.token);
    println!("Allowance:   ${}", status.allowance);
    println!("Spender:     {}", status.spender);
    println!();

    // Check if already approved for the requested amount
    if !status.needs_approval && status.allowance >= amount {
        println!("\u{2713} Already approved for ${} (current: ${})", amount, status.allowance);
        println!();
        return Ok(());
    }

    println!("Requested:   ${}", amount);
    println!();

    // Confirm unless skip_confirm is set
    if !skip_confirm {
        print!("Proceed with approval? [y/N] ");
        io::stdout().flush().ok();

        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            println!();
            return Ok(());
        }
        println!();
    }

    print!("Submitting transaction... ");
    io::stdout().flush().ok();

    let outcome = WalletService::approve(&config, amount).await?;

    match outcome {
        ApprovalOutcome::Approved { tx_hash, amount } => {
            println!("done");
            println!();
            println!("\u{2713} Approval successful");
            println!("  Amount:      ${amount}");
            println!("  Transaction: {tx_hash}");
            println!();
        }
        ApprovalOutcome::AlreadyApproved { current_allowance } => {
            println!("done");
            println!();
            println!("\u{2713} Already approved for ${current_allowance}");
            println!();
        }
        ApprovalOutcome::Failed { reason } => {
            println!("failed");
            println!();
            eprintln!("\u{2717} Approval failed: {reason}");
            println!();
            return Err(ExecutionError::OrderRejected(reason).into());
        }
    }

    Ok(())
}
