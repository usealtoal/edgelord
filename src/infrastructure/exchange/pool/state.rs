use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use tracing::warn;

use crate::domain::id::TokenId;

/// Shared counters updated atomically by connection and management tasks.
pub(super) struct SharedCounters {
    pub(super) rotations: AtomicU64,
    pub(super) restarts: AtomicU64,
    pub(super) events_dropped: AtomicU64,
}

impl SharedCounters {
    pub(super) fn new() -> Self {
        Self {
            rotations: AtomicU64::new(0),
            restarts: AtomicU64::new(0),
            events_dropped: AtomicU64::new(0),
        }
    }
}

/// Tracks the lifecycle of a single pooled connection.
pub(super) struct ConnectionState {
    /// Unique ID for logging and identification.
    pub(super) id: u64,
    /// Tokens this connection is responsible for.
    pub(super) tokens: Vec<TokenId>,
    /// When this connection was spawned.
    pub(super) spawned_at: Instant,
    /// Epoch millis of the last received event (updated atomically by the task).
    pub(super) last_event_at: Arc<AtomicU64>,
    /// Handle to the connection's tokio task.
    pub(super) handle: tokio::task::JoinHandle<()>,
}

/// Returns the current time as epoch milliseconds.
pub(super) fn epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Lock a mutex, recovering from poisoning.
///
/// If a thread panicked while holding the lock, we log a warning and recover
/// the data. This keeps the pool operational but surfaces the issue in logs.
pub(super) fn lock_or_recover<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            warn!("Mutex poisoned (previous holder panicked), recovering");
            poisoned.into_inner()
        }
    }
}
