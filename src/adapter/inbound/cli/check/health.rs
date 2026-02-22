use std::path::Path;

use crate::adapter::inbound::cli::{operator, output};
use crate::error::{Error, Result};
use crate::port::inbound::operator::diagnostic::HealthCheckStatus;
use serde_json::json;

/// Run a local health check using configuration.
pub fn execute_health<P: AsRef<Path>>(config_path: P) -> Result<()> {
    let config_toml = operator::read_config_toml(config_path.as_ref())?;
    let report = operator::operator().health_report(&config_toml)?;

    if output::is_json() {
        let checks = report
            .checks
            .iter()
            .map(|check| {
                let (status, details) = match &check.status {
                    HealthCheckStatus::Healthy => ("healthy", None),
                    HealthCheckStatus::Unhealthy(reason) => ("unhealthy", Some(reason.as_str())),
                };

                json!({
                    "name": check.name,
                    "critical": check.critical,
                    "status": status,
                    "details": details,
                })
            })
            .collect::<Vec<_>>();

        println!(
            "{}",
            json!({
                "command": "check.health",
                "status": if report.is_healthy() { "healthy" } else { "unhealthy" },
                "checks": checks,
            })
        );
    } else {
        output::section("Health Check");
        for check in &report.checks {
            let (status, details) = match &check.status {
                HealthCheckStatus::Healthy => ("healthy", None),
                HealthCheckStatus::Unhealthy(reason) => ("unhealthy", Some(reason)),
            };

            let suffix = if check.critical { " (critical)" } else { "" };
            output::field(
                &format!("{}{}", check.name, suffix),
                match details {
                    Some(reason) => format!("{status}: {reason}"),
                    None => status.to_string(),
                },
            );
        }
    }

    if output::is_quiet() && report.is_healthy() {
        return Ok(());
    }

    if !report.is_healthy() {
        output::error("Health check failed");
        return Err(Error::Connection("health check failed".to_string()));
    }
    output::success("Health check passed");
    Ok(())
}
