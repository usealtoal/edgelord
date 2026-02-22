//! Canonical test configurations.
//!
//! Single source of truth for config structs used across tests.
//! Avoids each test module defining its own slightly-different defaults.

use crate::infrastructure::config::service::{ConnectionPoolConfig, ReconnectionConfig};

/// Fast reconnection config with zero delays — no waiting in tests.
pub fn reconnection() -> ReconnectionConfig {
    ReconnectionConfig {
        initial_delay_ms: 0,
        max_delay_ms: 0,
        backoff_multiplier: 1.0,
        max_consecutive_failures: 3,
        circuit_breaker_cooldown_ms: 0,
    }
}

/// Connection pool config with specified limits.
///
/// Uses sensible defaults for TTL, health checks, and channel capacity.
/// For tests that need specific timing behavior, override individual fields
/// on the returned struct.
pub fn pool(max_connections: usize, subscriptions_per_connection: usize) -> ConnectionPoolConfig {
    ConnectionPoolConfig {
        max_connections,
        subscriptions_per_connection,
        connection_ttl_secs: 120,
        preemptive_reconnect_secs: 30,
        health_check_interval_secs: 30,
        max_silent_secs: 60,
        channel_capacity: 10_000,
    }
}
