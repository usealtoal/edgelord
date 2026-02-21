use std::path::Path;

use crate::app::{Config, Environment, Exchange};
use crate::cli::output;
use crate::error::{Error, Result};

/// Validate readiness for live trading.
pub fn execute_live<P: AsRef<Path>>(config_path: P) -> Result<()> {
    let config = Config::load(config_path.as_ref())?;
    let network = config.network();

    output::section("Live Readiness");
    output::field("Exchange", format!("{:?}", config.exchange));
    output::field("Environment", network.environment);
    output::field("Chain ID", network.chain_id);
    output::field("Dry run", config.dry_run);

    let mut blockers = Vec::new();

    match config.exchange {
        Exchange::Polymarket => {
            if network.environment != Environment::Mainnet {
                output::warning("Environment is not mainnet (expected mainnet)");
                blockers.push("environment");
            }
            if network.chain_id != 137 {
                output::warning("Chain ID is not Polygon mainnet (expected 137)");
                blockers.push("chain_id");
            }
            if config.wallet.private_key.is_none() {
                output::warning("Wallet not configured (set WALLET_PRIVATE_KEY or keystore)");
                blockers.push("wallet");
            }
            if config.dry_run {
                output::warning("Dry run is enabled (set dry_run=false for live trading)");
                blockers.push("dry_run");
            }
        }
    }

    if blockers.is_empty() {
        output::success("Ready for live trading");
        Ok(())
    } else {
        output::error("Live readiness check failed");
        Err(Error::Connection("live readiness check failed".to_string()))
    }
}
