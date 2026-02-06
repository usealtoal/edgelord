use std::path::Path;

use crate::app::{Config, Environment, Exchange};
use crate::error::{Error, Result};

/// Validate readiness for live trading.
pub fn execute_live<P: AsRef<Path>>(config_path: P) -> Result<()> {
    let config = Config::load(config_path.as_ref())?;
    let network = config.network();

    println!("Live readiness:");
    println!("  Exchange: {:?}", config.exchange);
    println!("  Environment: {}", network.environment);
    println!("  Chain ID: {}", network.chain_id);
    println!("  Dry-run: {}", config.dry_run);
    println!();

    let mut blockers = Vec::new();

    match config.exchange {
        Exchange::Polymarket => {
            if network.environment != Environment::Mainnet {
                println!("⚠ Environment is not mainnet (expected mainnet)");
                blockers.push("environment");
            }
            if network.chain_id != 137 {
                println!("⚠ Chain ID is not Polygon mainnet (expected 137)");
                blockers.push("chain_id");
            }
            if config.wallet.private_key.is_none() {
                println!("⚠ Wallet not configured (set WALLET_PRIVATE_KEY or keystore)");
                blockers.push("wallet");
            }
            if config.dry_run {
                println!("⚠ Dry-run is enabled (set dry_run=false for live trading)");
                blockers.push("dry_run");
            }
        }
    }

    if blockers.is_empty() {
        println!("✓ Ready for live trading");
        Ok(())
    } else {
        Err(Error::Connection("live readiness check failed".to_string()))
    }
}
