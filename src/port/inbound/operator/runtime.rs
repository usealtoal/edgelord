//! Runtime control projection types for operator-facing adapters.
//!
//! Defines request and response types for runtime control operations
//! such as starting the trading bot.

use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::error::Result;

/// Runtime configuration overrides from CLI flags.
///
/// Contains all parameters that can be overridden at runtime via command-line
/// arguments, taking precedence over the configuration file.
#[derive(Debug, Clone)]
pub struct RunRequest {
    /// Raw TOML configuration content.
    pub config_toml: String,

    /// Override for blockchain chain ID.
    pub chain_id: Option<u64>,

    /// Override for log level (e.g., "debug", "info", "warn").
    pub log_level: Option<String>,

    /// Whether to output logs as JSON.
    pub json_logs: bool,

    /// Override for enabled strategy names.
    pub strategies: Option<Vec<String>>,

    /// Override for minimum edge threshold.
    pub min_edge: Option<Decimal>,

    /// Override for minimum profit threshold.
    pub min_profit: Option<Decimal>,

    /// Override for maximum total exposure.
    pub max_exposure: Option<Decimal>,

    /// Override for maximum position per market.
    pub max_position: Option<Decimal>,

    /// Whether Telegram notifications are enabled.
    pub telegram_enabled: bool,

    /// Whether dry-run mode is enabled.
    pub dry_run: bool,

    /// Override for maximum slippage tolerance.
    pub max_slippage: Option<Decimal>,

    /// Override for execution timeout in seconds.
    pub execution_timeout: Option<u64>,

    /// Override for maximum markets to subscribe to.
    pub max_markets: Option<usize>,

    /// Override for minimum 24h volume filter.
    pub min_volume: Option<f64>,

    /// Override for minimum liquidity filter.
    pub min_liquidity: Option<f64>,

    /// Override for maximum WebSocket connections.
    pub max_connections: Option<usize>,

    /// Override for subscriptions per WebSocket connection.
    pub subscriptions_per_connection: Option<usize>,

    /// Override for connection TTL in seconds.
    pub connection_ttl_seconds: Option<u64>,

    /// Override for statistics reporting interval in seconds.
    pub stats_interval_seconds: Option<u64>,

    /// Override for database file path.
    pub database_path: Option<String>,

    /// Force mainnet environment.
    pub mainnet: bool,

    /// Force testnet environment.
    pub testnet: bool,
}

/// Startup information snapshot for display.
///
/// Contains the resolved configuration values shown at startup.
#[derive(Debug, Clone)]
pub struct RunStartupSnapshot {
    /// Human-readable network label (e.g., "mainnet (polygon)").
    pub network_label: String,

    /// Resolved blockchain chain ID.
    pub chain_id: u64,

    /// Masked wallet address for display.
    pub wallet_display: String,

    /// Names of enabled detection strategies.
    pub enabled_strategies: Vec<String>,

    /// Whether dry-run mode is active.
    pub dry_run: bool,
}

/// Runtime control use-cases for operator-facing adapters.
///
/// # Thread Safety
///
/// Implementations must be thread-safe (`Send + Sync`).
#[async_trait]
pub trait RuntimeOperator: Send + Sync {
    /// Prepare a startup snapshot from runtime overrides.
    ///
    /// # Arguments
    ///
    /// * `request` - Runtime configuration and overrides.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    fn prepare_run(&self, request: &RunRequest) -> Result<RunStartupSnapshot>;

    /// Execute the main runtime orchestration loop.
    ///
    /// # Arguments
    ///
    /// * `request` - Runtime configuration and overrides.
    ///
    /// # Errors
    ///
    /// Returns an error if runtime initialization or execution fails.
    async fn execute_run(&self, request: RunRequest) -> Result<()>;
}
