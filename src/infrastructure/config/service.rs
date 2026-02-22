//! Service configuration for wallet, risk, telegram, governor, and connection management.

use rust_decimal::Decimal;
use serde::Deserialize;

use crate::application::state::RiskLimits;

/// Wallet configuration for signing orders.
/// Private key is loaded from `WALLET_PRIVATE_KEY` env var at runtime (never from config file).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct WalletConfig {
    /// Optional keystore path for encrypted wallet storage.
    #[serde(default)]
    pub keystore_path: Option<String>,
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
    /// Execution timeout in seconds (default: 30).
    #[serde(default = "default_execution_timeout_secs")]
    pub execution_timeout_secs: u64,
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

const fn default_execution_timeout_secs() -> u64 {
    30
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_position_per_market: default_max_position_per_market(),
            max_total_exposure: default_max_total_exposure(),
            min_profit_threshold: default_min_profit_threshold(),
            max_slippage: default_max_slippage(),
            execution_timeout_secs: default_execution_timeout_secs(),
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
            execution_timeout_secs: config.execution_timeout_secs,
        }
    }
}

const fn default_true() -> bool {
    true
}

/// Telegram notification configuration.
#[derive(Debug, Clone, Deserialize)]
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
    /// Stats polling interval in seconds (default: 30).
    #[serde(default = "default_stats_interval_secs")]
    pub stats_interval_secs: u64,
    /// Maximum positions to display (default: 10).
    #[serde(default = "default_position_display_limit")]
    pub position_display_limit: usize,
}

const fn default_stats_interval_secs() -> u64 {
    30
}

const fn default_position_display_limit() -> usize {
    10
}

impl Default for TelegramAppConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            notify_opportunities: false,
            notify_executions: default_true(),
            notify_risk_rejections: default_true(),
            stats_interval_secs: default_stats_interval_secs(),
            position_display_limit: default_position_display_limit(),
        }
    }
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

/// Connection pool configuration for WebSocket multiplexing.
///
/// These settings control how multiple WebSocket connections are managed
/// to distribute subscriptions across connections.
#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionPoolConfig {
    /// Maximum number of connections in the pool.
    #[serde(default = "default_pool_max_connections")]
    pub max_connections: usize,
    /// Maximum subscriptions per connection.
    #[serde(default = "default_pool_subscriptions_per_connection")]
    pub subscriptions_per_connection: usize,
    /// Connection time-to-live in seconds.
    #[serde(default = "default_pool_connection_ttl_secs")]
    pub connection_ttl_secs: u64,
    /// Seconds before TTL to preemptively reconnect.
    #[serde(default = "default_pool_preemptive_reconnect_secs")]
    pub preemptive_reconnect_secs: u64,
    /// Health check interval in seconds.
    #[serde(default = "default_pool_health_check_interval_secs")]
    pub health_check_interval_secs: u64,
    /// Maximum seconds with no events before considering a connection unhealthy.
    #[serde(default = "default_pool_max_silent_secs")]
    pub max_silent_secs: u64,
    /// Event channel capacity (bounded to prevent unbounded memory growth).
    #[serde(default = "default_pool_channel_capacity")]
    pub channel_capacity: usize,
}

const fn default_pool_max_connections() -> usize {
    10
}

const fn default_pool_subscriptions_per_connection() -> usize {
    500
}

const fn default_pool_connection_ttl_secs() -> u64 {
    120
}

const fn default_pool_preemptive_reconnect_secs() -> u64 {
    30
}

const fn default_pool_health_check_interval_secs() -> u64 {
    30
}

const fn default_pool_max_silent_secs() -> u64 {
    60
}

const fn default_pool_channel_capacity() -> usize {
    10_000
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_connections: default_pool_max_connections(),
            subscriptions_per_connection: default_pool_subscriptions_per_connection(),
            connection_ttl_secs: default_pool_connection_ttl_secs(),
            preemptive_reconnect_secs: default_pool_preemptive_reconnect_secs(),
            health_check_interval_secs: default_pool_health_check_interval_secs(),
            max_silent_secs: default_pool_max_silent_secs(),
            channel_capacity: default_pool_channel_capacity(),
        }
    }
}
