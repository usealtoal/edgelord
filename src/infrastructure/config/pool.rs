//! Connection pool and reconnection configuration.
//!
//! Provides configuration for WebSocket connection pooling and automatic
//! reconnection with exponential backoff.

use serde::Deserialize;

/// WebSocket reconnection configuration.
///
/// Controls automatic reconnection behavior with exponential backoff and
/// circuit breaker protection against repeated failures.
#[derive(Debug, Clone, Deserialize)]
pub struct ReconnectionConfig {
    /// Initial delay before first reconnection attempt in milliseconds.
    ///
    /// Defaults to 1000 (1 second).
    #[serde(default = "default_initial_delay_ms")]
    pub initial_delay_ms: u64,

    /// Maximum delay between reconnection attempts in milliseconds.
    ///
    /// The backoff delay is capped at this value. Defaults to 60000 (60 seconds).
    #[serde(default = "default_max_delay_ms")]
    pub max_delay_ms: u64,

    /// Multiplier applied to delay after each failed attempt.
    ///
    /// Creates exponential backoff. Must be >= 1.0. Defaults to 2.0.
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,

    /// Maximum consecutive failures before circuit breaker trips.
    ///
    /// After this many failures, reconnection attempts are paused.
    /// Defaults to 10.
    #[serde(default = "default_max_consecutive_failures")]
    pub max_consecutive_failures: u32,

    /// Cooldown period after circuit breaker trips in milliseconds.
    ///
    /// Duration to wait before resuming reconnection attempts.
    /// Defaults to 300000 (5 minutes).
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
/// Controls how multiple WebSocket connections are managed to distribute
/// subscriptions across connections. This helps avoid per-connection
/// subscription limits imposed by exchanges.
#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionPoolConfig {
    /// Maximum number of connections in the pool.
    ///
    /// Subscriptions are distributed across up to this many connections.
    /// Defaults to 10.
    #[serde(default = "default_pool_max_connections")]
    pub max_connections: usize,

    /// Maximum subscriptions per connection.
    ///
    /// When a connection reaches this limit, new subscriptions use a
    /// different connection. Defaults to 500.
    #[serde(default = "default_pool_subscriptions_per_connection")]
    pub subscriptions_per_connection: usize,

    /// Connection time-to-live in seconds.
    ///
    /// Connections are rotated after this duration to prevent stale state.
    /// Defaults to 120 (2 minutes).
    #[serde(default = "default_pool_connection_ttl_secs")]
    pub connection_ttl_secs: u64,

    /// Seconds before TTL to begin preemptive reconnection.
    ///
    /// New connections are established this many seconds before TTL expiry
    /// to enable zero-gap handoff. Defaults to 30.
    #[serde(default = "default_pool_preemptive_reconnect_secs")]
    pub preemptive_reconnect_secs: u64,

    /// Health check interval in seconds.
    ///
    /// How often the management task checks connection health.
    /// Defaults to 30.
    #[serde(default = "default_pool_health_check_interval_secs")]
    pub health_check_interval_secs: u64,

    /// Maximum seconds with no events before considering a connection unhealthy.
    ///
    /// Silent connections are restarted. Defaults to 60.
    #[serde(default = "default_pool_max_silent_secs")]
    pub max_silent_secs: u64,

    /// Event channel capacity.
    ///
    /// Size of the bounded channel that buffers events from all connections.
    /// Bounded to prevent unbounded memory growth under backpressure.
    /// Defaults to 10000.
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
