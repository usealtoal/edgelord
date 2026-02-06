use std::path::Path;

use crate::app::{health_check, Config, HealthStatus};
use crate::error::{Error, Result};

/// Run a local health check using configuration.
pub fn execute_health<P: AsRef<Path>>(config_path: P) -> Result<()> {
    let config = Config::load(config_path.as_ref())?;
    let report = health_check(&config);

    println!("Health check:");
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
        return Err(Error::Connection("health check failed".to_string()));
    }
    println!("✓ Health check passed");
    Ok(())
}
