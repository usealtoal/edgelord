//! Diagnostic projections for operator-facing adapters.

use async_trait::async_trait;

use crate::error::Result;

/// Summary output for `check config`.
#[derive(Debug, Clone)]
pub struct ConfigCheckReport {
    pub exchange: String,
    pub environment: String,
    pub chain_id: u64,
    pub enabled_strategies: Vec<String>,
    pub dry_run: bool,
    pub wallet_configured: bool,
    pub telegram_enabled: bool,
    pub telegram_token_present: bool,
    pub telegram_chat_present: bool,
}

/// Live-readiness output for `check live`.
#[derive(Debug, Clone)]
pub struct LiveReadinessReport {
    pub exchange: String,
    pub environment: String,
    pub chain_id: u64,
    pub dry_run: bool,
    pub environment_is_mainnet: bool,
    pub chain_is_polygon_mainnet: bool,
    pub wallet_configured: bool,
}

impl LiveReadinessReport {
    /// True when no live-trading blockers are present.
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        self.environment_is_mainnet
            && self.chain_is_polygon_mainnet
            && self.wallet_configured
            && !self.dry_run
    }
}

/// Connection endpoints for connectivity checks.
#[derive(Debug, Clone)]
pub struct ConnectionCheckTarget {
    pub exchange: String,
    pub environment: String,
    pub ws_url: String,
    pub api_url: String,
}

/// Health status item for operator checks.
#[derive(Debug, Clone)]
pub enum HealthCheckStatus {
    Healthy,
    Unhealthy(String),
}

/// Individual health check entry.
#[derive(Debug, Clone)]
pub struct HealthCheckEntry {
    pub name: String,
    pub critical: bool,
    pub status: HealthCheckStatus,
}

/// Health check report projection.
#[derive(Debug, Clone, Default)]
pub struct HealthCheckReport {
    pub checks: Vec<HealthCheckEntry>,
}

impl HealthCheckReport {
    /// True when all critical checks are healthy.
    #[must_use]
    pub fn is_healthy(&self) -> bool {
        self.checks
            .iter()
            .filter(|check| check.critical)
            .all(|check| matches!(&check.status, HealthCheckStatus::Healthy))
    }
}

/// Output from sending a Telegram test message.
#[derive(Debug, Clone)]
pub struct TelegramTestReceipt {
    pub masked_token: String,
    pub chat_id: String,
}

/// Diagnostics use-cases for operator-facing adapters.
#[async_trait]
pub trait DiagnosticOperator: Send + Sync {
    /// Build `check config` summary.
    fn check_config(&self, config_toml: &str) -> Result<ConfigCheckReport>;

    /// Build live-readiness report.
    fn check_live_readiness(&self, config_toml: &str) -> Result<LiveReadinessReport>;

    /// Resolve exchange connection endpoints for checks.
    fn connection_target(&self, config_toml: &str) -> Result<ConnectionCheckTarget>;

    /// Check REST endpoint reachability.
    async fn verify_rest_connectivity(&self, api_url: &str) -> Result<()>;

    /// Check WebSocket endpoint reachability.
    async fn verify_websocket_connectivity(&self, ws_url: &str) -> Result<()>;

    /// Run a local health check.
    fn health_report(&self, config_toml: &str) -> Result<HealthCheckReport>;

    /// Send a Telegram test message.
    async fn send_telegram_test(&self, config_toml: &str) -> Result<TelegramTestReceipt>;
}
