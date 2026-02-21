use std::path::Path;

use crate::cli::output;
use crate::cli::wallet::WalletService;
use crate::error::Result;
use crate::runtime::Config;

/// Show the wallet address derived from the configured key material.
pub fn execute_address(config_path: &Path) -> Result<()> {
    let config = Config::load(config_path)?;
    let address = WalletService::wallet_address(&config)?;

    output::section("Wallet Address");
    output::field("Address", address);
    Ok(())
}
