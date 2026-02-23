//! Runtime control projections for operator-facing adapters.

use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::error::Result;

/// Runtime overrides captured from CLI flags.
#[derive(Debug, Clone)]
pub struct RunRequest {
    pub config_toml: String,
    pub chain_id: Option<u64>,
    pub log_level: Option<String>,
    pub json_logs: bool,
    pub strategies: Option<Vec<String>>,
    pub min_edge: Option<Decimal>,
    pub min_profit: Option<Decimal>,
    pub max_exposure: Option<Decimal>,
    pub max_position: Option<Decimal>,
    pub telegram_enabled: bool,
    pub dry_run: bool,
    pub max_slippage: Option<Decimal>,
    pub execution_timeout: Option<u64>,
    pub max_markets: Option<usize>,
    pub min_volume: Option<f64>,
    pub min_liquidity: Option<f64>,
    pub max_connections: Option<usize>,
    pub subscriptions_per_connection: Option<usize>,
    pub connection_ttl_seconds: Option<u64>,
    pub stats_interval_seconds: Option<u64>,
    pub database_path: Option<String>,
    pub mainnet: bool,
    pub testnet: bool,
}

/// Startup display projection for `run`.
#[derive(Debug, Clone)]
pub struct RunStartupSnapshot {
    pub network_label: String,
    pub chain_id: u64,
    pub wallet_display: String,
    pub enabled_strategies: Vec<String>,
    pub dry_run: bool,
}

/// Runtime use-cases for operator-facing adapters.
#[async_trait]
pub trait RuntimeOperator: Send + Sync {
    /// Build startup snapshot from run overrides.
    fn prepare_run(&self, request: &RunRequest) -> Result<RunStartupSnapshot>;

    /// Execute runtime orchestration using run overrides.
    async fn execute_run(&self, request: RunRequest) -> Result<()>;
}
