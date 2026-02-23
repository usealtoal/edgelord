//! Connection pool internal state types.
//!
//! Provides shared state and helper types used by the connection pool
//! and its management task.

use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use tracing::warn;

use crate::domain::id::TokenId;

/// Shared counters updated atomically by connection and management tasks.
///
/// Provides observability metrics for the connection pool.
pub(super) struct SharedCounters {
    /// Total number of TTL-based connection rotations.
    pub(super) rotations: AtomicU64,
    /// Total number of connection restarts due to crashes or silent death.
    pub(super) restarts: AtomicU64,
    /// Total number of events dropped due to channel backpressure.
    pub(super) events_dropped: AtomicU64,
}

impl SharedCounters {
    /// Create a new set of zeroed counters.
    pub(super) fn new() -> Self {
        Self {
            rotations: AtomicU64::new(0),
            restarts: AtomicU64::new(0),
            events_dropped: AtomicU64::new(0),
        }
    }
}

/// Tracks the lifecycle of a single pooled connection.
///
/// Contains all state needed to monitor and manage a connection.
pub(super) struct ConnectionState {
    /// Unique ID for logging and identification.
    pub(super) id: u64,
    /// Token IDs this connection is responsible for.
    pub(super) tokens: Vec<TokenId>,
    /// Instant when this connection was spawned.
    pub(super) spawned_at: Instant,
    /// Epoch milliseconds of the last received event.
    ///
    /// Updated atomically by the connection task.
    pub(super) last_event_at: Arc<AtomicU64>,
    /// Handle to the connection's tokio task.
    pub(super) handle: tokio::task::JoinHandle<()>,
}

/// Return the current time as epoch milliseconds.
pub(super) fn epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Lock a mutex, recovering from poisoning if necessary.
///
/// If a thread panicked while holding the lock, logs a warning and recovers
/// the data. This keeps the pool operational while surfacing the issue.
pub(super) fn lock_or_recover<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            warn!("Mutex poisoned (previous holder panicked), recovering");
            poisoned.into_inner()
        }
    }
}
