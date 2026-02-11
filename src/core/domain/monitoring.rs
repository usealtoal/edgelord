//! Monitoring and observability data types.
//!
//! These types represent runtime statistics and health metrics
//! that are exchange-agnostic and can be used across layers.

/// Runtime statistics for a connection pool.
///
/// Used for observability and monitoring (e.g., Telegram `/pool` command).
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    /// Number of currently active connections.
    pub active_connections: usize,
    /// Total number of connection rotations (TTL-triggered).
    pub total_rotations: u64,
    /// Total number of restarts (crash/silence-triggered).
    pub total_restarts: u64,
    /// Total number of events dropped due to a full channel.
    pub events_dropped: u64,
}
