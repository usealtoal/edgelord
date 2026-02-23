//! Polymarket exchange configuration.

use serde::Deserialize;

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

/// Connection management settings for Polymarket streams.
#[derive(Debug, Clone, Deserialize)]
pub struct PolymarketConnectionConfig {
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
    /// Event channel capacity.
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

impl Default for PolymarketConnectionConfig {
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

/// Polymarket HTTP client configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct PolymarketHttpConfig {
    /// Request timeout in milliseconds.
    #[serde(default = "default_http_timeout_ms")]
    pub timeout_ms: u64,
    /// Connect timeout in milliseconds.
    #[serde(default = "default_http_connect_timeout_ms")]
    pub connect_timeout_ms: u64,
    /// Maximum number of retry attempts for transient failures.
    #[serde(default = "default_http_retry_max_attempts")]
    pub retry_max_attempts: u32,
    /// Backoff between retries in milliseconds.
    #[serde(default = "default_http_retry_backoff_ms")]
    pub retry_backoff_ms: u64,
}

const fn default_http_timeout_ms() -> u64 {
    5000
}

const fn default_http_connect_timeout_ms() -> u64 {
    2000
}

const fn default_http_retry_max_attempts() -> u32 {
    3
}

const fn default_http_retry_backoff_ms() -> u64 {
    500
}

impl Default for PolymarketHttpConfig {
    fn default() -> Self {
        Self {
            timeout_ms: default_http_timeout_ms(),
            connect_timeout_ms: default_http_connect_timeout_ms(),
            retry_max_attempts: default_http_retry_max_attempts(),
            retry_backoff_ms: default_http_retry_backoff_ms(),
        }
    }
}

/// Polymarket market filter configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct PolymarketFilterConfig {
    /// Maximum number of markets to track.
    #[serde(default = "default_filter_max_markets")]
    pub max_markets: usize,
    /// Maximum number of subscriptions.
    #[serde(default = "default_filter_max_subscriptions")]
    pub max_subscriptions: usize,
    /// Minimum 24-hour volume threshold.
    #[serde(default = "default_min_volume_24h")]
    pub min_volume_24h: f64,
    /// Minimum liquidity threshold.
    #[serde(default = "default_min_liquidity")]
    pub min_liquidity: f64,
    /// Maximum spread percentage (e.g., 0.10 = 10%).
    #[serde(default = "default_max_spread_pct")]
    pub max_spread_pct: f64,
    /// Include binary (two-outcome) markets.
    #[serde(default = "default_true")]
    pub include_binary: bool,
    /// Include multi-outcome markets.
    #[serde(default = "default_true")]
    pub include_multi_outcome: bool,
    /// Maximum number of outcomes per market.
    #[serde(default = "default_max_outcomes")]
    pub max_outcomes: usize,
}

const fn default_true() -> bool {
    true
}

const fn default_filter_max_markets() -> usize {
    500
}

const fn default_filter_max_subscriptions() -> usize {
    2000
}

fn default_min_volume_24h() -> f64 {
    1000.0
}

fn default_min_liquidity() -> f64 {
    500.0
}

fn default_max_spread_pct() -> f64 {
    0.10
}

const fn default_max_outcomes() -> usize {
    20
}

impl Default for PolymarketFilterConfig {
    fn default() -> Self {
        Self {
            max_markets: default_filter_max_markets(),
            max_subscriptions: default_filter_max_subscriptions(),
            min_volume_24h: default_min_volume_24h(),
            min_liquidity: default_min_liquidity(),
            max_spread_pct: default_max_spread_pct(),
            include_binary: true,
            include_multi_outcome: true,
            max_outcomes: default_max_outcomes(),
        }
    }
}

/// Scoring weights for market prioritization.
#[derive(Debug, Clone, Deserialize)]
pub struct ScoringWeightsConfig {
    /// Weight for liquidity factor.
    #[serde(default = "default_weight_liquidity")]
    pub liquidity: f64,
    /// Weight for spread factor.
    #[serde(default = "default_weight_spread")]
    pub spread: f64,
    /// Weight for opportunity factor.
    #[serde(default = "default_weight_opportunity")]
    pub opportunity: f64,
    /// Weight for outcome count factor.
    #[serde(default = "default_weight_outcome_count")]
    pub outcome_count: f64,
    /// Weight for activity factor.
    #[serde(default = "default_weight_activity")]
    pub activity: f64,
}

fn default_weight_liquidity() -> f64 {
    0.0 // Disabled until Phase 2 (needs order book data)
}

fn default_weight_spread() -> f64 {
    0.0 // Disabled until Phase 2 (needs order book data)
}

fn default_weight_opportunity() -> f64 {
    0.50 // Primary factor - calculated from price imbalance
}

fn default_weight_outcome_count() -> f64 {
    0.40 // Secondary factor - more outcomes = more arbitrage potential
}

fn default_weight_activity() -> f64 {
    0.10 // Placeholder until Phase 2 (needs trade data)
}

impl Default for ScoringWeightsConfig {
    fn default() -> Self {
        Self {
            liquidity: default_weight_liquidity(),
            spread: default_weight_spread(),
            opportunity: default_weight_opportunity(),
            outcome_count: default_weight_outcome_count(),
            activity: default_weight_activity(),
        }
    }
}

/// Outcome count bonus configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct OutcomeBonusConfig {
    /// Bonus multiplier for binary (2-outcome) markets.
    #[serde(default = "default_bonus_binary")]
    pub binary: f64,
    /// Bonus multiplier for 3-5 outcome markets.
    #[serde(default = "default_bonus_three_to_five")]
    pub three_to_five: f64,
    /// Bonus multiplier for 6+ outcome markets.
    #[serde(default = "default_bonus_six_plus")]
    pub six_plus: f64,
}

fn default_bonus_binary() -> f64 {
    1.0
}

fn default_bonus_three_to_five() -> f64 {
    1.5
}

fn default_bonus_six_plus() -> f64 {
    2.0
}

impl Default for OutcomeBonusConfig {
    fn default() -> Self {
        Self {
            binary: default_bonus_binary(),
            three_to_five: default_bonus_three_to_five(),
            six_plus: default_bonus_six_plus(),
        }
    }
}

/// Polymarket scoring configuration.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PolymarketScoringConfig {
    /// Scoring weights for market prioritization.
    #[serde(default)]
    pub weights: ScoringWeightsConfig,
    /// Outcome count bonus configuration.
    #[serde(default)]
    pub outcome_bonus: OutcomeBonusConfig,
}

/// Deduplication strategy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DedupStrategyConfig {
    /// Hash-based deduplication (default).
    #[default]
    Hash,
    /// Timestamp-based deduplication.
    Timestamp,
    /// Content-based deduplication.
    Content,
}

/// Polymarket deduplication configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct PolymarketDedupConfig {
    /// Enable deduplication.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Primary deduplication strategy.
    #[serde(default)]
    pub strategy: DedupStrategyConfig,
    /// Fallback deduplication strategy.
    #[serde(default = "default_fallback_strategy")]
    pub fallback: DedupStrategyConfig,
    /// Cache time-to-live in seconds.
    #[serde(default = "default_dedup_cache_ttl_secs")]
    pub cache_ttl_secs: u64,
    /// Maximum cache entries.
    #[serde(default = "default_max_cache_entries")]
    pub max_cache_entries: usize,
}

fn default_fallback_strategy() -> DedupStrategyConfig {
    DedupStrategyConfig::Timestamp
}

const fn default_dedup_cache_ttl_secs() -> u64 {
    5
}

const fn default_max_cache_entries() -> usize {
    100_000
}

impl Default for PolymarketDedupConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            strategy: DedupStrategyConfig::default(),
            fallback: default_fallback_strategy(),
            cache_ttl_secs: default_dedup_cache_ttl_secs(),
            max_cache_entries: default_max_cache_entries(),
        }
    }
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
    /// CLOB REST API URL (order execution, order book queries).
    #[serde(default = "default_polymarket_api_url")]
    pub api_url: String,
    /// Gamma REST API URL (market discovery, volume/liquidity metadata).
    #[serde(default = "default_polymarket_gamma_url")]
    pub gamma_api_url: String,
    /// Chain ID: 80002 for Amoy testnet, 137 for Polygon mainnet.
    #[serde(default = "default_polymarket_chain_id")]
    pub chain_id: u64,
    /// Connection management configuration.
    #[serde(default)]
    pub connections: PolymarketConnectionConfig,
    /// HTTP client configuration for REST API calls.
    #[serde(default)]
    pub http: PolymarketHttpConfig,
    /// Market filter configuration.
    #[serde(default)]
    pub market_filter: PolymarketFilterConfig,
    /// Scoring configuration for market prioritization.
    #[serde(default)]
    pub scoring: PolymarketScoringConfig,
    /// Deduplication configuration.
    #[serde(default)]
    pub dedup: PolymarketDedupConfig,
}

fn default_polymarket_ws_url() -> String {
    "wss://ws-subscriptions-clob.polymarket.com/ws/market".into()
}

fn default_polymarket_api_url() -> String {
    "https://clob.polymarket.com".into()
}

fn default_polymarket_gamma_url() -> String {
    "https://gamma-api.polymarket.com".into()
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
            gamma_api_url: default_polymarket_gamma_url(),
            chain_id: default_polymarket_chain_id(),
            connections: PolymarketConnectionConfig::default(),
            http: PolymarketHttpConfig::default(),
            market_filter: PolymarketFilterConfig::default(),
            scoring: PolymarketScoringConfig::default(),
            dedup: PolymarketDedupConfig::default(),
        }
    }
}

/// Runtime credentials and network settings required by signing adapters.
#[derive(Debug, Clone)]
pub struct PolymarketRuntimeConfig {
    /// Wallet private key (hex, no 0x prefix).
    pub private_key: String,
    /// Chain ID for signature domain separation.
    pub chain_id: u64,
    /// CLOB API base URL.
    pub api_url: String,
    /// Deployment environment.
    pub environment: Environment,
}
