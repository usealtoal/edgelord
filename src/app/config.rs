//! Application configuration loading and validation.
//!
//! Configuration is loaded from a TOML file with environment variable overrides
//! for sensitive values like `WALLET_PRIVATE_KEY`.

use rust_decimal::Decimal;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tracing_subscriber::{fmt, EnvFilter};

use crate::app::state::RiskLimits;
use crate::core::domain::ResourceBudget;
use crate::core::strategy::{CombinatorialConfig, MarketRebalancingConfig, SingleConditionConfig};
use crate::error::{ConfigError, Result};

/// Application profile for resource allocation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    /// Local development with conservative resource usage.
    #[default]
    Local,
    /// Production with higher resource capacity.
    Production,
    /// Custom profile using explicit ResourceConfig values.
    Custom,
}

/// Resource configuration for adaptive subscription management.
#[derive(Debug, Clone, Deserialize)]
pub struct ResourceConfig {
    /// Auto-detect system resources at startup.
    #[serde(default)]
    pub auto_detect: bool,
    /// Maximum memory budget in megabytes.
    #[serde(default)]
    pub max_memory_mb: Option<u64>,
    /// Number of worker threads.
    #[serde(default)]
    pub worker_threads: Option<usize>,
    /// Target memory utilization (0.0-1.0).
    #[serde(default = "default_memory_usage_target")]
    pub memory_usage_target: f64,
    /// Target CPU utilization (0.0-1.0).
    #[serde(default = "default_cpu_usage_target")]
    pub cpu_usage_target: f64,
}

fn default_memory_usage_target() -> f64 {
    0.80
}

fn default_cpu_usage_target() -> f64 {
    0.70
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            auto_detect: false,
            max_memory_mb: None,
            worker_threads: None,
            memory_usage_target: default_memory_usage_target(),
            cpu_usage_target: default_cpu_usage_target(),
        }
    }
}

impl ResourceConfig {
    /// Convert to a ResourceBudget, using auto-detection if enabled.
    #[must_use]
    pub fn to_budget(&self, profile: Profile) -> ResourceBudget {
        // Start with profile-based defaults
        let base = match profile {
            Profile::Local => ResourceBudget::local(),
            Profile::Production => ResourceBudget::production(),
            Profile::Custom => ResourceBudget::local(), // Start with local as base for custom
        };

        // Determine memory bytes
        let max_memory_bytes = if let Some(mb) = self.max_memory_mb {
            mb * 1024 * 1024
        } else if self.auto_detect {
            // Use system memory if auto-detect enabled (fallback to base)
            base.max_memory_bytes
        } else {
            base.max_memory_bytes
        };

        // Determine worker threads
        let worker_threads = if let Some(threads) = self.worker_threads {
            threads
        } else if self.auto_detect {
            num_cpus::get()
        } else {
            base.worker_threads
        };

        ResourceBudget::new(
            max_memory_bytes,
            worker_threads,
            self.memory_usage_target,
            self.cpu_usage_target,
        )
    }
}

/// Supported exchanges.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Exchange {
    #[default]
    Polymarket,
}

/// Exchange-specific configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ExchangeSpecificConfig {
    Polymarket(PolymarketConfig),
}

impl Default for ExchangeSpecificConfig {
    fn default() -> Self {
        Self::Polymarket(PolymarketConfig::default())
    }
}

/// Exchange environment (testnet vs mainnet).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    #[default]
    Testnet,
    Mainnet,
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Testnet => write!(f, "testnet"),
            Self::Mainnet => write!(f, "mainnet"),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    /// Application profile for resource allocation.
    #[serde(default)]
    pub profile: Profile,
    /// Resource configuration for adaptive subscription management.
    #[serde(default)]
    pub resources: ResourceConfig,
    /// Which exchange to connect to.
    #[serde(default)]
    pub exchange: Exchange,
    /// Exchange-specific configuration.
    #[serde(default, alias = "polymarket")]
    pub exchange_config: ExchangeSpecificConfig,
    pub logging: LoggingConfig,
    #[serde(default)]
    pub strategies: StrategiesConfig,
    #[serde(default)]
    pub wallet: WalletConfig,
    #[serde(default)]
    pub risk: RiskConfig,
    #[serde(default)]
    pub telegram: TelegramAppConfig,
    /// Governor configuration for adaptive subscription management.
    #[serde(default)]
    pub governor: GovernorAppConfig,
    /// Dry-run mode: detect opportunities but don't execute trades.
    #[serde(default)]
    pub dry_run: bool,
    /// Path to the status file for external monitoring.
    /// Set to enable status file writing (e.g., "/var/run/edgelord/status.json").
    #[serde(default)]
    pub status_file: Option<PathBuf>,
    #[serde(default)]
    pub reconnection: ReconnectionConfig,
    /// Connection pool configuration for WebSocket shard management.
    #[serde(default)]
    pub connection_pool: ConnectionPoolConfig,
}

/// Configuration for all detection strategies.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct StrategiesConfig {
    /// Enabled strategy names.
    #[serde(default = "default_enabled_strategies")]
    pub enabled: Vec<String>,

    /// Single-condition strategy config.
    #[serde(default)]
    pub single_condition: SingleConditionConfig,

    /// Market rebalancing strategy config.
    #[serde(default)]
    pub market_rebalancing: MarketRebalancingConfig,

    /// Combinatorial (Frank-Wolfe + ILP) strategy config.
    #[serde(default)]
    pub combinatorial: CombinatorialConfig,
}

fn default_enabled_strategies() -> Vec<String> {
    vec!["single_condition".to_string()]
}

/// Wallet configuration for signing orders.
/// Private key is loaded from `WALLET_PRIVATE_KEY` env var at runtime (never from config file).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct WalletConfig {
    /// Private key loaded from `WALLET_PRIVATE_KEY` env var at runtime
    #[serde(skip)]
    pub private_key: Option<String>,
}

/// Risk management configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct RiskConfig {
    /// Maximum position size per market in dollars.
    #[serde(default = "default_max_position_per_market")]
    pub max_position_per_market: Decimal,
    /// Maximum total exposure across all positions.
    #[serde(default = "default_max_total_exposure")]
    pub max_total_exposure: Decimal,
    /// Minimum profit threshold to execute.
    #[serde(default = "default_min_profit_threshold")]
    pub min_profit_threshold: Decimal,
    /// Maximum slippage tolerance (e.g., 0.02 = 2%).
    #[serde(default = "default_max_slippage")]
    pub max_slippage: Decimal,
}

fn default_max_position_per_market() -> Decimal {
    Decimal::from(1000)
}

fn default_max_total_exposure() -> Decimal {
    Decimal::from(10000)
}

fn default_min_profit_threshold() -> Decimal {
    Decimal::new(5, 2) // $0.05
}

fn default_max_slippage() -> Decimal {
    Decimal::new(2, 2) // 2%
}

const fn default_true() -> bool {
    true
}

// Governor configuration defaults
const fn default_target_p50_ms() -> u64 {
    10
}

const fn default_target_p95_ms() -> u64 {
    50
}

const fn default_target_p99_ms() -> u64 {
    100
}

const fn default_max_p99_ms() -> u64 {
    200
}

const fn default_check_interval_secs() -> u64 {
    10
}

fn default_expand_threshold() -> f64 {
    0.70
}

fn default_contract_threshold() -> f64 {
    1.20
}

const fn default_expand_step() -> usize {
    50
}

const fn default_contract_step() -> usize {
    100
}

const fn default_cooldown_secs() -> u64 {
    60
}

/// Latency target configuration for the governor.
#[derive(Debug, Clone, Deserialize)]
pub struct LatencyTargetsConfig {
    /// Target p50 latency in milliseconds.
    #[serde(default = "default_target_p50_ms")]
    pub target_p50_ms: u64,
    /// Target p95 latency in milliseconds.
    #[serde(default = "default_target_p95_ms")]
    pub target_p95_ms: u64,
    /// Target p99 latency in milliseconds.
    #[serde(default = "default_target_p99_ms")]
    pub target_p99_ms: u64,
    /// Maximum acceptable p99 latency in milliseconds.
    #[serde(default = "default_max_p99_ms")]
    pub max_p99_ms: u64,
}

impl Default for LatencyTargetsConfig {
    fn default() -> Self {
        Self {
            target_p50_ms: default_target_p50_ms(),
            target_p95_ms: default_target_p95_ms(),
            target_p99_ms: default_target_p99_ms(),
            max_p99_ms: default_max_p99_ms(),
        }
    }
}

/// Scaling configuration for the governor.
#[derive(Debug, Clone, Deserialize)]
pub struct ScalingAppConfig {
    /// Interval between scaling checks in seconds.
    #[serde(default = "default_check_interval_secs")]
    pub check_interval_secs: u64,
    /// Utilization threshold below which to expand subscriptions.
    #[serde(default = "default_expand_threshold")]
    pub expand_threshold: f64,
    /// Utilization threshold above which to contract subscriptions.
    #[serde(default = "default_contract_threshold")]
    pub contract_threshold: f64,
    /// Number of subscriptions to add when expanding.
    #[serde(default = "default_expand_step")]
    pub expand_step: usize,
    /// Number of subscriptions to remove when contracting.
    #[serde(default = "default_contract_step")]
    pub contract_step: usize,
    /// Cooldown period between scaling actions in seconds.
    #[serde(default = "default_cooldown_secs")]
    pub cooldown_secs: u64,
}

impl Default for ScalingAppConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: default_check_interval_secs(),
            contract_threshold: default_contract_threshold(),
            expand_threshold: default_expand_threshold(),
            expand_step: default_expand_step(),
            contract_step: default_contract_step(),
            cooldown_secs: default_cooldown_secs(),
        }
    }
}

/// Governor configuration for adaptive subscription management.
#[derive(Debug, Clone, Deserialize)]
pub struct GovernorAppConfig {
    /// Enable the governor for adaptive scaling.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Latency target configuration.
    #[serde(default)]
    pub latency: LatencyTargetsConfig,
    /// Scaling configuration.
    #[serde(default)]
    pub scaling: ScalingAppConfig,
}

impl Default for GovernorAppConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            latency: LatencyTargetsConfig::default(),
            scaling: ScalingAppConfig::default(),
        }
    }
}

/// Telegram notification configuration.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TelegramAppConfig {
    /// Enable telegram notifications.
    #[serde(default)]
    pub enabled: bool,
    /// Send opportunity alerts (can be noisy).
    #[serde(default)]
    pub notify_opportunities: bool,
    /// Send execution alerts.
    #[serde(default = "default_true")]
    pub notify_executions: bool,
    /// Send risk rejection alerts.
    #[serde(default = "default_true")]
    pub notify_risk_rejections: bool,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_position_per_market: default_max_position_per_market(),
            max_total_exposure: default_max_total_exposure(),
            min_profit_threshold: default_min_profit_threshold(),
            max_slippage: default_max_slippage(),
        }
    }
}

/// Connection pool configuration for WebSocket shard management.
#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionPoolConfig {
    /// Number of shards to distribute connections across.
    #[serde(default = "default_num_shards")]
    pub num_shards: usize,
    /// Number of connections per shard.
    #[serde(default = "default_connections_per_shard")]
    pub connections_per_shard: usize,
    /// Stagger offset in seconds between connection attempts.
    #[serde(default = "default_stagger_offset_secs")]
    pub stagger_offset_secs: u64,
    /// Health check interval in seconds.
    #[serde(default = "default_health_check_interval_secs")]
    pub health_check_interval_secs: u64,
    /// Maximum silent period in seconds before considering connection unhealthy.
    #[serde(default = "default_max_silent_secs")]
    pub max_silent_secs: u64,
}

const fn default_num_shards() -> usize {
    3
}

const fn default_connections_per_shard() -> usize {
    2
}

const fn default_stagger_offset_secs() -> u64 {
    60
}

const fn default_health_check_interval_secs() -> u64 {
    5
}

const fn default_max_silent_secs() -> u64 {
    10
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            num_shards: default_num_shards(),
            connections_per_shard: default_connections_per_shard(),
            stagger_offset_secs: default_stagger_offset_secs(),
            health_check_interval_secs: default_health_check_interval_secs(),
            max_silent_secs: default_max_silent_secs(),
        }
    }
}

impl ConnectionPoolConfig {
    /// Configuration for local development with minimal resources.
    #[must_use]
    pub fn local() -> Self {
        Self {
            num_shards: 1,
            connections_per_shard: 1,
            ..Self::default()
        }
    }

    /// Configuration for production with default scaling.
    #[must_use]
    pub fn production() -> Self {
        Self::default()
    }
}

/// WebSocket reconnection configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ReconnectionConfig {
    /// Initial delay before first reconnection attempt (milliseconds).
    #[serde(default = "default_initial_delay_ms")]
    pub initial_delay_ms: u64,
    /// Maximum delay between reconnection attempts (milliseconds).
    #[serde(default = "default_max_delay_ms")]
    pub max_delay_ms: u64,
    /// Multiplier applied to delay after each failed attempt.
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,
    /// Maximum consecutive failures before circuit breaker trips.
    #[serde(default = "default_max_consecutive_failures")]
    pub max_consecutive_failures: u32,
    /// Cooldown period after circuit breaker trips (milliseconds).
    #[serde(default = "default_circuit_breaker_cooldown_ms")]
    pub circuit_breaker_cooldown_ms: u64,
}

fn default_initial_delay_ms() -> u64 {
    1000 // 1 second
}

fn default_max_delay_ms() -> u64 {
    60000 // 60 seconds
}

fn default_backoff_multiplier() -> f64 {
    2.0
}

fn default_max_consecutive_failures() -> u32 {
    10
}

fn default_circuit_breaker_cooldown_ms() -> u64 {
    300000 // 5 minutes
}

impl Default for ReconnectionConfig {
    fn default() -> Self {
        Self {
            initial_delay_ms: default_initial_delay_ms(),
            max_delay_ms: default_max_delay_ms(),
            backoff_multiplier: default_backoff_multiplier(),
            max_consecutive_failures: default_max_consecutive_failures(),
            circuit_breaker_cooldown_ms: default_circuit_breaker_cooldown_ms(),
        }
    }
}

impl From<RiskConfig> for RiskLimits {
    fn from(config: RiskConfig) -> Self {
        Self {
            max_position_per_market: config.max_position_per_market,
            max_total_exposure: config.max_total_exposure,
            min_profit_threshold: config.min_profit_threshold,
            max_slippage: config.max_slippage,
        }
    }
}

/// Common network configuration returned by exchanges.
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub environment: Environment,
    pub ws_url: String,
    pub api_url: String,
    pub chain_id: u64,
}

/// Polymarket exchange configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct PolymarketConfig {
    /// Environment: testnet or mainnet.
    #[serde(default)]
    pub environment: Environment,
    /// WebSocket URL for market data.
    #[serde(default = "default_polymarket_ws_url")]
    pub ws_url: String,
    /// REST API URL.
    #[serde(default = "default_polymarket_api_url")]
    pub api_url: String,
    /// Chain ID: 80002 for Amoy testnet, 137 for Polygon mainnet.
    #[serde(default = "default_polymarket_chain_id")]
    pub chain_id: u64,
}

fn default_polymarket_ws_url() -> String {
    "wss://ws-subscriptions-clob.polymarket.com/ws/market".into()
}

fn default_polymarket_api_url() -> String {
    "https://clob.polymarket.com".into()
}

/// Default chain ID is Amoy testnet (80002) for safety
const fn default_polymarket_chain_id() -> u64 {
    80002
}

impl Default for PolymarketConfig {
    fn default() -> Self {
        Self {
            environment: Environment::default(),
            ws_url: default_polymarket_ws_url(),
            api_url: default_polymarket_api_url(),
            chain_id: default_polymarket_chain_id(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
}

impl Config {
    #[allow(clippy::result_large_err)]
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(ConfigError::ReadFile)?;

        let mut config: Self = toml::from_str(&content).map_err(ConfigError::Parse)?;

        // Load private key from environment variable (never from config file for security)
        config.wallet.private_key = std::env::var("WALLET_PRIVATE_KEY").ok();

        config.validate()?;

        Ok(config)
    }

    #[allow(clippy::result_large_err)]
    fn validate(&self) -> Result<()> {
        let network = self.network();
        if network.ws_url.is_empty() {
            return Err(ConfigError::MissingField { field: "ws_url" }.into());
        }
        if network.api_url.is_empty() {
            return Err(ConfigError::MissingField { field: "api_url" }.into());
        }
        Ok(())
    }

    /// Get the network configuration for the active exchange.
    #[must_use]
    pub fn network(&self) -> NetworkConfig {
        match &self.exchange_config {
            ExchangeSpecificConfig::Polymarket(poly) => NetworkConfig {
                environment: poly.environment,
                ws_url: poly.ws_url.clone(),
                api_url: poly.api_url.clone(),
                chain_id: poly.chain_id,
            },
        }
    }

    /// Get Polymarket-specific config if this is a Polymarket exchange.
    #[must_use]
    pub fn polymarket_config(&self) -> Option<&PolymarketConfig> {
        match &self.exchange_config {
            ExchangeSpecificConfig::Polymarket(config) => Some(config),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            profile: Profile::default(),
            resources: ResourceConfig::default(),
            exchange: Exchange::default(),
            exchange_config: ExchangeSpecificConfig::default(),
            logging: LoggingConfig {
                level: "info".into(),
                format: "pretty".into(),
            },
            strategies: StrategiesConfig::default(),
            wallet: WalletConfig::default(),
            risk: RiskConfig::default(),
            telegram: TelegramAppConfig::default(),
            governor: GovernorAppConfig::default(),
            dry_run: false,
            status_file: None,
            reconnection: ReconnectionConfig::default(),
            connection_pool: ConnectionPoolConfig::default(),
        }
    }
}

impl Config {
    pub fn init_logging(&self) {
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(&self.logging.level));

        match self.logging.format.as_str() {
            "json" => {
                fmt().json().with_env_filter(filter).init();
            }
            _ => {
                fmt().with_env_filter(filter).init();
            }
        }
    }
}
