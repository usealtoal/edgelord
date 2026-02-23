use std::path::Path;

use crate::adapter::inbound::cli::{operator, output};
use crate::error::Result;

/// Show the wallet address derived from the configured key material.
pub fn execute_address(config_path: &Path) -> Result<()> {
    let config_toml = operator::read_config_toml(config_path)?;
    let address = operator::operator().wallet_address(&config_toml)?;

    output::section("Wallet Address");
    output::field("Address", address);
    Ok(())
}
