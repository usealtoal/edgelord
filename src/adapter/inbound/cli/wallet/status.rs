use std::path::Path;

use crate::adapter::inbound::cli::{operator, output};
use crate::error::Result;

/// Display current wallet approval status.
///
/// Shows the current token allowance and whether additional approval is needed.
pub async fn execute_status(config_path: &Path) -> Result<()> {
    output::section("Wallet Status");
    let config_toml = operator::read_config_toml(config_path)?;

    let service = operator::operator();
    let pb = output::spinner("Fetching approval status");
    let status = match service.wallet_status(&config_toml).await {
        Ok(status) => {
            output::spinner_success(&pb, "Fetching approval status");
            status
        }
        Err(e) => {
            output::spinner_fail(&pb, "Fetching approval status");
            return Err(e);
        }
    };
    output::field("Exchange", status.exchange);
    output::field("Wallet", status.wallet_address);
    output::field("Token", status.token);
    output::field("Allowance", format!("${}", status.allowance));
    output::field("Spender", status.spender);

    if status.needs_approval {
        output::warning("Approval required");
        println!("  Run `edgelord wallet approve` to approve token spending.");
    } else {
        output::success("Token approval is in place");
    }

    Ok(())
}
