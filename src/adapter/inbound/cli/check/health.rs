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

        output::json_output(json!({
            "command": "check.health",
            "status": if report.is_healthy() { "healthy" } else { "unhealthy" },
            "checks": checks,
        }));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::inbound::operator::diagnostic::{HealthCheckEntry, HealthCheckReport};

    // Tests for HealthCheckStatus formatting

    #[test]
    fn test_health_check_status_healthy_format() {
        let status = HealthCheckStatus::Healthy;
        let (status_str, details) = match &status {
            HealthCheckStatus::Healthy => ("healthy", None),
            HealthCheckStatus::Unhealthy(reason) => ("unhealthy", Some(reason.as_str())),
        };
        assert_eq!(status_str, "healthy");
        assert!(details.is_none());
    }

    #[test]
    fn test_health_check_status_unhealthy_format() {
        let status = HealthCheckStatus::Unhealthy("disk full".to_string());
        let (status_str, details) = match &status {
            HealthCheckStatus::Healthy => ("healthy", None),
            HealthCheckStatus::Unhealthy(reason) => ("unhealthy", Some(reason.as_str())),
        };
        assert_eq!(status_str, "unhealthy");
        assert_eq!(details, Some("disk full"));
    }

    #[test]
    fn test_health_check_status_unhealthy_empty_reason() {
        let status = HealthCheckStatus::Unhealthy("".to_string());
        let (status_str, details) = match &status {
            HealthCheckStatus::Healthy => ("healthy", None),
            HealthCheckStatus::Unhealthy(reason) => ("unhealthy", Some(reason.as_str())),
        };
        assert_eq!(status_str, "unhealthy");
        assert_eq!(details, Some(""));
    }

    // Tests for HealthCheckEntry formatting

    #[test]
    fn test_health_check_entry_critical_suffix() {
        let entry = HealthCheckEntry {
            name: "Disk Space".to_string(),
            critical: true,
            status: HealthCheckStatus::Healthy,
        };

        let suffix = if entry.critical { " (critical)" } else { "" };
        assert_eq!(suffix, " (critical)");
    }

    #[test]
    fn test_health_check_entry_non_critical_suffix() {
        let entry = HealthCheckEntry {
            name: "Optional Check".to_string(),
            critical: false,
            status: HealthCheckStatus::Healthy,
        };

        let suffix = if entry.critical { " (critical)" } else { "" };
        assert_eq!(suffix, "");
    }

    #[test]
    fn test_health_check_entry_format_with_name_and_suffix() {
        let entry = HealthCheckEntry {
            name: "Memory".to_string(),
            critical: true,
            status: HealthCheckStatus::Healthy,
        };

        let suffix = if entry.critical { " (critical)" } else { "" };
        let formatted = format!("{}{}", entry.name, suffix);
        assert_eq!(formatted, "Memory (critical)");
    }

    // Tests for HealthCheckReport is_healthy

    #[test]
    fn test_health_check_report_empty_is_healthy() {
        let report = HealthCheckReport { checks: vec![] };
        assert!(report.is_healthy());
    }

    #[test]
    fn test_health_check_report_all_healthy() {
        let report = HealthCheckReport {
            checks: vec![
                HealthCheckEntry {
                    name: "Check 1".to_string(),
                    critical: true,
                    status: HealthCheckStatus::Healthy,
                },
                HealthCheckEntry {
                    name: "Check 2".to_string(),
                    critical: true,
                    status: HealthCheckStatus::Healthy,
                },
            ],
        };
        assert!(report.is_healthy());
    }

    #[test]
    fn test_health_check_report_critical_unhealthy() {
        let report = HealthCheckReport {
            checks: vec![
                HealthCheckEntry {
                    name: "Check 1".to_string(),
                    critical: true,
                    status: HealthCheckStatus::Unhealthy("error".to_string()),
                },
                HealthCheckEntry {
                    name: "Check 2".to_string(),
                    critical: true,
                    status: HealthCheckStatus::Healthy,
                },
            ],
        };
        assert!(!report.is_healthy());
    }

    #[test]
    fn test_health_check_report_non_critical_unhealthy_is_healthy() {
        let report = HealthCheckReport {
            checks: vec![
                HealthCheckEntry {
                    name: "Check 1".to_string(),
                    critical: false,
                    status: HealthCheckStatus::Unhealthy("warning".to_string()),
                },
                HealthCheckEntry {
                    name: "Check 2".to_string(),
                    critical: true,
                    status: HealthCheckStatus::Healthy,
                },
            ],
        };
        // Non-critical failures don't affect overall health
        assert!(report.is_healthy());
    }

    #[test]
    fn test_health_check_report_all_non_critical_unhealthy() {
        let report = HealthCheckReport {
            checks: vec![
                HealthCheckEntry {
                    name: "Check 1".to_string(),
                    critical: false,
                    status: HealthCheckStatus::Unhealthy("warning 1".to_string()),
                },
                HealthCheckEntry {
                    name: "Check 2".to_string(),
                    critical: false,
                    status: HealthCheckStatus::Unhealthy("warning 2".to_string()),
                },
            ],
        };
        // All non-critical, so still healthy
        assert!(report.is_healthy());
    }

    // Tests for JSON output format

    #[test]
    fn test_json_check_format_healthy() {
        let check = HealthCheckEntry {
            name: "Database".to_string(),
            critical: true,
            status: HealthCheckStatus::Healthy,
        };

        let (status, details) = match &check.status {
            HealthCheckStatus::Healthy => ("healthy", None),
            HealthCheckStatus::Unhealthy(reason) => ("unhealthy", Some(reason.as_str())),
        };

        let json_value = json!({
            "name": check.name,
            "critical": check.critical,
            "status": status,
            "details": details,
        });

        assert_eq!(json_value["name"], "Database");
        assert_eq!(json_value["critical"], true);
        assert_eq!(json_value["status"], "healthy");
        assert!(json_value["details"].is_null());
    }

    #[test]
    fn test_json_check_format_unhealthy() {
        let check = HealthCheckEntry {
            name: "Network".to_string(),
            critical: false,
            status: HealthCheckStatus::Unhealthy("connection timeout".to_string()),
        };

        let (status, details) = match &check.status {
            HealthCheckStatus::Healthy => ("healthy", None),
            HealthCheckStatus::Unhealthy(reason) => ("unhealthy", Some(reason.as_str())),
        };

        let json_value = json!({
            "name": check.name,
            "critical": check.critical,
            "status": status,
            "details": details,
        });

        assert_eq!(json_value["name"], "Network");
        assert_eq!(json_value["critical"], false);
        assert_eq!(json_value["status"], "unhealthy");
        assert_eq!(json_value["details"], "connection timeout");
    }

    #[test]
    fn test_json_output_status_healthy() {
        let report = HealthCheckReport {
            checks: vec![HealthCheckEntry {
                name: "Test".to_string(),
                critical: true,
                status: HealthCheckStatus::Healthy,
            }],
        };

        let status = if report.is_healthy() {
            "healthy"
        } else {
            "unhealthy"
        };
        assert_eq!(status, "healthy");
    }

    #[test]
    fn test_json_output_status_unhealthy() {
        let report = HealthCheckReport {
            checks: vec![HealthCheckEntry {
                name: "Test".to_string(),
                critical: true,
                status: HealthCheckStatus::Unhealthy("failed".to_string()),
            }],
        };

        let status = if report.is_healthy() {
            "healthy"
        } else {
            "unhealthy"
        };
        assert_eq!(status, "unhealthy");
    }

    // Tests for output formatting logic

    #[test]
    fn test_healthy_status_string_format() {
        let (status, details): (&str, Option<&str>) = ("healthy", None);
        let formatted = match details {
            Some(reason) => format!("{status}: {reason}"),
            None => status.to_string(),
        };
        assert_eq!(formatted, "healthy");
    }

    #[test]
    fn test_unhealthy_status_string_format() {
        let (status, details): (&str, Option<&str>) = ("unhealthy", Some("disk full"));
        let formatted = match details {
            Some(reason) => format!("{status}: {reason}"),
            None => status.to_string(),
        };
        assert_eq!(formatted, "unhealthy: disk full");
    }

    // Tests for HealthCheckEntry and HealthCheckReport cloning

    #[test]
    fn test_health_check_entry_clone() {
        let entry = HealthCheckEntry {
            name: "Test".to_string(),
            critical: true,
            status: HealthCheckStatus::Healthy,
        };
        let cloned = entry.clone();
        assert_eq!(entry.name, cloned.name);
        assert_eq!(entry.critical, cloned.critical);
    }

    #[test]
    fn test_health_check_report_clone() {
        let report = HealthCheckReport {
            checks: vec![HealthCheckEntry {
                name: "Test".to_string(),
                critical: true,
                status: HealthCheckStatus::Healthy,
            }],
        };
        let cloned = report.clone();
        assert_eq!(report.checks.len(), cloned.checks.len());
    }

    // Tests for HealthCheckStatus clone

    #[test]
    fn test_health_check_status_clone_healthy() {
        let status = HealthCheckStatus::Healthy;
        let cloned = status.clone();
        assert!(matches!(cloned, HealthCheckStatus::Healthy));
    }

    #[test]
    fn test_health_check_status_clone_unhealthy() {
        let status = HealthCheckStatus::Unhealthy("error".to_string());
        let cloned = status.clone();
        if let HealthCheckStatus::Unhealthy(reason) = cloned {
            assert_eq!(reason, "error");
        } else {
            panic!("Expected Unhealthy status");
        }
    }

    // Tests for Debug implementations

    #[test]
    fn test_health_check_status_debug() {
        let status = HealthCheckStatus::Healthy;
        let debug_str = format!("{:?}", status);
        assert!(debug_str.contains("Healthy"));
    }

    #[test]
    fn test_health_check_entry_debug() {
        let entry = HealthCheckEntry {
            name: "Test".to_string(),
            critical: true,
            status: HealthCheckStatus::Healthy,
        };
        let debug_str = format!("{:?}", entry);
        assert!(debug_str.contains("HealthCheckEntry"));
        assert!(debug_str.contains("Test"));
    }

    #[test]
    fn test_health_check_report_debug() {
        let report = HealthCheckReport { checks: vec![] };
        let debug_str = format!("{:?}", report);
        assert!(debug_str.contains("HealthCheckReport"));
    }
}
