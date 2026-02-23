//! Runtime health reporting.

use crate::infrastructure::config::settings::Config;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Unhealthy(String),
}

#[derive(Debug, Clone)]
pub struct HealthCheck {
    name: &'static str,
    critical: bool,
    status: HealthStatus,
}

impl HealthCheck {
    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn critical(&self) -> bool {
        self.critical
    }

    pub fn status(&self) -> &HealthStatus {
        &self.status
    }

    pub fn is_healthy(&self) -> bool {
        matches!(self.status, HealthStatus::Healthy)
    }
}

#[derive(Debug, Clone)]
pub struct HealthReport {
    checks: Vec<HealthCheck>,
}

impl HealthReport {
    pub fn checks(&self) -> &[HealthCheck] {
        &self.checks
    }

    pub fn is_healthy(&self) -> bool {
        self.checks
            .iter()
            .filter(|check| check.critical())
            .all(HealthCheck::is_healthy)
    }
}

pub fn health_check(config: &Config) -> HealthReport {
    let network = config.network();
    let mut checks = Vec::new();

    checks.push(HealthCheck {
        name: "database",
        critical: true,
        status: if config.database.trim().is_empty() {
            HealthStatus::Unhealthy("database path is empty".to_string())
        } else {
            HealthStatus::Healthy
        },
    });

    checks.push(HealthCheck {
        name: "exchange_api",
        critical: true,
        status: if network.api_url.trim().is_empty() {
            HealthStatus::Unhealthy("api_url is empty".to_string())
        } else {
            HealthStatus::Healthy
        },
    });

    checks.push(HealthCheck {
        name: "exchange_ws",
        critical: true,
        status: if network.ws_url.trim().is_empty() {
            HealthStatus::Unhealthy("ws_url is empty".to_string())
        } else {
            HealthStatus::Healthy
        },
    });

    checks.push(HealthCheck {
        name: "strategies",
        critical: true,
        status: if config.strategies.enabled.is_empty() {
            HealthStatus::Unhealthy("no strategies enabled".to_string())
        } else {
            HealthStatus::Healthy
        },
    });

    HealthReport { checks }
}

#[cfg(test)]
mod tests {
    use super::{health_check, HealthCheck, HealthReport, HealthStatus};
    use crate::infrastructure::config::settings::{Config, ExchangeSpecificConfig};

    #[test]
    fn health_check_struct_accessors() {
        let check = HealthCheck {
            name: "test_service",
            critical: true,
            status: HealthStatus::Healthy,
        };

        assert_eq!(check.name(), "test_service");
        assert!(check.critical());
        assert!(matches!(check.status(), HealthStatus::Healthy));
        assert!(check.is_healthy());
    }

    #[test]
    fn health_check_unhealthy_status() {
        let check = HealthCheck {
            name: "broken_service",
            critical: false,
            status: HealthStatus::Unhealthy("connection failed".to_string()),
        };

        assert!(!check.is_healthy());
        assert!(matches!(check.status(), HealthStatus::Unhealthy(_)));
    }

    #[test]
    fn health_report_is_healthy_when_all_critical_pass() {
        let report = HealthReport {
            checks: vec![
                HealthCheck {
                    name: "critical_pass",
                    critical: true,
                    status: HealthStatus::Healthy,
                },
                HealthCheck {
                    name: "non_critical_fail",
                    critical: false,
                    status: HealthStatus::Unhealthy("warning".to_string()),
                },
            ],
        };

        assert!(report.is_healthy());
    }

    #[test]
    fn health_report_is_unhealthy_when_critical_fails() {
        let report = HealthReport {
            checks: vec![
                HealthCheck {
                    name: "critical_fail",
                    critical: true,
                    status: HealthStatus::Unhealthy("error".to_string()),
                },
                HealthCheck {
                    name: "critical_pass",
                    critical: true,
                    status: HealthStatus::Healthy,
                },
            ],
        };

        assert!(!report.is_healthy());
    }

    #[test]
    fn health_report_checks_accessor() {
        let report = HealthReport {
            checks: vec![
                HealthCheck {
                    name: "check1",
                    critical: true,
                    status: HealthStatus::Healthy,
                },
                HealthCheck {
                    name: "check2",
                    critical: false,
                    status: HealthStatus::Healthy,
                },
            ],
        };

        assert_eq!(report.checks().len(), 2);
    }

    #[test]
    fn health_check_with_default_config() {
        let config = Config::default();
        let report = health_check(&config);

        assert!(report.checks().len() >= 4);

        let check_names: Vec<_> = report.checks().iter().map(|c| c.name()).collect();
        assert!(check_names.contains(&"database"));
        assert!(check_names.contains(&"exchange_api"));
        assert!(check_names.contains(&"exchange_ws"));
        assert!(check_names.contains(&"strategies"));
    }

    #[test]
    fn health_check_detects_empty_database_path() {
        let config = Config {
            database: String::new(),
            ..Default::default()
        };

        let report = health_check(&config);
        let db_check = report
            .checks()
            .iter()
            .find(|c| c.name() == "database")
            .unwrap();

        assert!(!db_check.is_healthy());
    }

    #[test]
    fn health_check_detects_empty_api_url() {
        let mut config = Config::default();
        match &mut config.exchange_config {
            ExchangeSpecificConfig::Polymarket(pm) => {
                pm.api_url = String::new();
            }
        }

        let report = health_check(&config);
        let api_check = report
            .checks()
            .iter()
            .find(|c| c.name() == "exchange_api")
            .unwrap();

        assert!(!api_check.is_healthy());
    }

    #[test]
    fn health_check_detects_empty_ws_url() {
        let mut config = Config::default();
        match &mut config.exchange_config {
            ExchangeSpecificConfig::Polymarket(pm) => {
                pm.ws_url = String::new();
            }
        }

        let report = health_check(&config);
        let ws_check = report
            .checks()
            .iter()
            .find(|c| c.name() == "exchange_ws")
            .unwrap();

        assert!(!ws_check.is_healthy());
    }

    #[test]
    fn health_check_detects_no_strategies_enabled() {
        let mut config = Config::default();
        config.strategies.enabled.clear();

        let report = health_check(&config);
        let strat_check = report
            .checks()
            .iter()
            .find(|c| c.name() == "strategies")
            .unwrap();

        assert!(!strat_check.is_healthy());
    }

    #[test]
    fn health_status_equality() {
        assert_eq!(HealthStatus::Healthy, HealthStatus::Healthy);
        assert_eq!(
            HealthStatus::Unhealthy("a".to_string()),
            HealthStatus::Unhealthy("a".to_string())
        );
        assert_ne!(
            HealthStatus::Healthy,
            HealthStatus::Unhealthy("error".to_string())
        );
    }
}
