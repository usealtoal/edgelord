use std::path::Path;

use crate::app::{Config, WalletService};
use crate::cli::output;
use crate::error::Result;

/// Display current wallet approval status.
///
/// Shows the current token allowance and whether additional approval is needed.
pub async fn execute_status(config_path: &Path) -> Result<()> {
    let config = Config::load(config_path)?;

    output::section("Wallet Status");

    output::progress("Fetching approval status");
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

    if status.needs_approval {
        output::warn("Approval required");
        output::note("Run `edgelord wallet approve` to approve token spending.");
    } else {
        output::ok("Token approval is in place");
    }

    Ok(())
}
