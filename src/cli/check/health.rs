use std::path::Path;

use crate::cli::output;
use crate::error::{Error, Result};
use crate::runtime::{health_check, Config, HealthStatus};

/// Run a local health check using configuration.
pub fn execute_health<P: AsRef<Path>>(config_path: P) -> Result<()> {
    let config = Config::load(config_path.as_ref())?;
    let report = health_check(&config);

    output::section("Health Check");
    for check in report.checks() {
        let status = match check.status() {
            HealthStatus::Healthy => "✓",
            HealthStatus::Unhealthy(_) => "✗",
        };
        println!(
            "  {status} {}{}",
            check.name(),
            if check.critical() { " (critical)" } else { "" }
        );
    }

    if !report.is_healthy() {
        output::error("Health check failed");
        return Err(Error::Connection("health check failed".to_string()));
    }
    output::success("Health check passed");
    Ok(())
}
