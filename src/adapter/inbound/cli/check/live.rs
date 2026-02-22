use std::path::Path;

use crate::adapter::inbound::cli::{operator, output};
use crate::error::{Error, Result};

/// Validate readiness for live trading.
pub fn execute_live<P: AsRef<Path>>(config_path: P) -> Result<()> {
    let config_toml = operator::read_config_toml(config_path.as_ref())?;
    let report = operator::operator().check_live_readiness(&config_toml)?;

    output::section("Live Readiness");
    output::field("Exchange", &report.exchange);
    output::field("Environment", &report.environment);
    output::field("Chain ID", report.chain_id);
    output::field("Dry run", report.dry_run);

    if !report.environment_is_mainnet {
        output::warning("Environment is not mainnet (expected mainnet)");
    }
    if !report.chain_is_polygon_mainnet {
        output::warning("Chain ID is not Polygon mainnet (expected 137)");
    }
    if !report.wallet_configured {
        output::warning("Wallet not configured (set WALLET_PRIVATE_KEY or keystore)");
    }
    if report.dry_run {
        output::warning("Dry run is enabled (set dry_run=false for live trading)");
    }

    if report.is_ready() {
        output::success("Ready for live trading");
        Ok(())
    } else {
        output::error("Live readiness check failed");
        Err(Error::Connection("live readiness check failed".to_string()))
    }
}
