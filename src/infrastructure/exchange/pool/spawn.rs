use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::mpsc;
use tracing::{debug, error, warn};

use crate::domain::id::TokenId;
use crate::infrastructure::config::pool::ReconnectionConfig;
use crate::infrastructure::exchange::reconnecting::ReconnectingDataStream;
use crate::port::outbound::exchange::{MarketDataStream, MarketEvent};

use super::state::{epoch_millis, ConnectionState, SharedCounters};
use super::StreamFactory;

/// Spawn a connection task that reads events and forwards them to `event_tx`.
///
/// This is a free function (not a method) so both the pool and the management
/// task can call it without borrow conflicts on `self`.
fn spawn_connection(
    factory: &StreamFactory,
    reconnection_config: ReconnectionConfig,
    tokens: Vec<TokenId>,
    event_tx: mpsc::Sender<MarketEvent>,
    connection_id: u64,
    last_event_at: Arc<AtomicU64>,
    counters: Arc<SharedCounters>,
) -> tokio::task::JoinHandle<()> {
    let stream = factory();
    let mut stream = ReconnectingDataStream::new(stream, reconnection_config);

    tokio::spawn(async move {
        let token_count = tokens.len();
        debug!(
            connection_id,
            tokens = token_count,
            "Connection task starting"
        );

        if let Err(e) = stream.connect().await {
            error!(connection_id, error = %e, "Failed to connect");
            return;
        }
        if let Err(e) = stream.subscribe(&tokens).await {
            error!(connection_id, error = %e, "Failed to subscribe");
            return;
        }

        debug!(connection_id, tokens = token_count, "Subscribed");

        loop {
            match stream.next_event().await {
                Some(event) => {
                    last_event_at.store(epoch_millis(), Ordering::Relaxed);
                    match event_tx.try_send(event) {
                        Ok(()) => {}
                        Err(mpsc::error::TrySendError::Full(_)) => {
                            counters.events_dropped.fetch_add(1, Ordering::Relaxed);
                            warn!(connection_id, "Event channel full — dropping event");
                        }
                        Err(mpsc::error::TrySendError::Closed(_)) => {
                            debug!(connection_id, "Channel closed, terminating");
                            break;
                        }
                    }
                }
                None => {
                    warn!(connection_id, "Stream ended");
                    break;
                }
            }
        }

        debug!(connection_id, "Task terminated");
    })
}

/// Build a fresh [`ConnectionState`], spawning its task immediately.
///
/// `last_event_at` is initialized to the current timestamp so that the
/// silent-death detector doesn't flag a brand-new connection that hasn't
/// received its first event yet.
pub(super) fn new_connection(
    id: u64,
    tokens: Vec<TokenId>,
    factory: &StreamFactory,
    reconnection_config: ReconnectionConfig,
    event_tx: mpsc::Sender<MarketEvent>,
    counters: Arc<SharedCounters>,
) -> ConnectionState {
    let last_event_at = Arc::new(AtomicU64::new(epoch_millis()));
    let handle = spawn_connection(
        factory,
        reconnection_config,
        tokens.clone(),
        event_tx,
        id,
        last_event_at.clone(),
        counters,
    );
    ConnectionState {
        id,
        tokens,
        spawned_at: Instant::now(),
        last_event_at,
        handle,
    }
}
