use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::infrastructure::config::service::{ConnectionPoolConfig, ReconnectionConfig};
use crate::port::outbound::exchange::MarketEvent;

use super::spawn::new_connection;
use super::state::{epoch_millis, lock_or_recover, ConnectionState, SharedCounters};
use super::{StreamFactory, DRAIN_GRACE_PERIOD, HANDOFF_POLL_INTERVAL};

/// Shared resources passed to the management and replacement tasks.
///
/// Bundles all the dependencies that `replace_connection` needs, avoiding
/// long parameter lists and making it easy to add new shared state.
pub(super) struct ManagementContext {
    pub(super) connections: Arc<Mutex<Vec<ConnectionState>>>,
    pub(super) config: ConnectionPoolConfig,
    pub(super) reconnection_config: ReconnectionConfig,
    pub(super) factory: StreamFactory,
    pub(super) event_tx: mpsc::Sender<MarketEvent>,
    pub(super) counters: Arc<SharedCounters>,
}

/// Descriptor for a connection that needs replacement.
pub(super) struct ReplacementJob {
    pub(super) index: usize,
    pub(super) reason: ReplacementReason,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum ReplacementReason {
    Ttl,
    Silent,
    Crashed,
}

impl ReplacementReason {
    pub(super) fn is_rotation(self) -> bool {
        matches!(self, Self::Ttl)
    }
}

/// Wait for a connection's first event, returning true on success.
async fn await_handoff(state: &ConnectionState, initial_ts: u64, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        tokio::time::sleep(HANDOFF_POLL_INTERVAL).await;
        if state.last_event_at.load(Ordering::Relaxed) > initial_ts {
            return true;
        }
        if state.handle.is_finished() {
            warn!(connection_id = state.id, "Replacement died during handoff");
            return false;
        }
        if Instant::now() >= deadline {
            warn!(
                connection_id = state.id,
                "Handoff timeout — swapping anyway"
            );
            return true; // old connection is stale, swap regardless
        }
    }
}

/// Replace a single connection: spawn, handoff, drain, swap.
pub(super) async fn replace_connection(
    ctx: &ManagementContext,
    index: usize,
    reason: ReplacementReason,
    new_id: u64,
    handoff_timeout: Duration,
) {
    // Read tokens from the existing connection.
    let tokens = {
        let conns = lock_or_recover(&ctx.connections);
        match conns.get(index) {
            Some(c) => c.tokens.clone(),
            None => {
                warn!(index, "Connection index out of bounds, skipping");
                return;
            }
        }
    };

    // Capture timestamp before spawning so we can detect the first event
    // from the replacement, even if it arrives before we start polling.
    // Subtract 1ms to handle same-millisecond races between capture and spawn.
    let initial_ts = epoch_millis().saturating_sub(1);

    let state = new_connection(
        new_id,
        tokens,
        &ctx.factory,
        ctx.reconnection_config.clone(),
        ctx.event_tx.clone(),
        ctx.counters.clone(),
    );
    if !await_handoff(&state, initial_ts, handoff_timeout).await {
        state.handle.abort();
        return; // management will retry next tick
    }

    // Swap: extract old handle under lock, then drain + abort outside lock.
    let swap_result = {
        let mut conns = lock_or_recover(&ctx.connections);
        if index < conns.len() {
            let old_id = conns[index].id;
            let old_handle = std::mem::replace(&mut conns[index], state).handle;
            Some((old_id, old_handle))
        } else {
            state.handle.abort();
            warn!(index, "Connection index shifted, skipping swap");
            None
        }
    }; // MutexGuard dropped here, before any .await

    if let Some((old_id, old_handle)) = swap_result {
        // Graceful drain: let old connection flush in-flight events.
        tokio::time::sleep(DRAIN_GRACE_PERIOD).await;
        old_handle.abort();

        if reason.is_rotation() {
            ctx.counters.rotations.fetch_add(1, Ordering::Relaxed);
            info!(
                old_connection_id = old_id,
                new_connection_id = new_id,
                "TTL rotation complete"
            );
        } else {
            ctx.counters.restarts.fetch_add(1, Ordering::Relaxed);
            info!(old_connection_id = old_id, new_connection_id = new_id, reason = ?reason, "Restart complete");
        }
    }
}
