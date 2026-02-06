use std::io::{self, Write};
use std::path::Path;

use crate::app::{Config, WalletService};
use crate::error::Result;

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
