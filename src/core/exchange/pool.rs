//! Connection pool for managing multiple WebSocket connections.
//!
//! This module provides an exchange-agnostic connection pool that distributes
//! subscriptions across multiple WebSocket connections to avoid hitting
//! per-connection subscription limits.
//!
//! # Architecture
//!
//! Each connection runs as a separate tokio task that reads from its WebSocket
//! and sends events into a shared `mpsc` channel. The pool merges events from
//! all connections into a single stream via `next_event()`.
//!
//! A background management task monitors all connections for:
//! - **TTL expiry**: proactively replaces connections before they hit their lifetime limit
//! - **Silent death**: detects connections that are "connected" but not receiving events
//! - **Task crashes**: restarts connections whose tasks have unexpectedly finished
//!
//! Replacements are preemptive — new connections are spawned and subscribed
//! before old ones are aborted, ensuring zero-gap event flow.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::app::{ConnectionPoolConfig, ReconnectionConfig};
use crate::core::domain::TokenId;
use crate::core::exchange::{MarketDataStream, MarketEvent, ReconnectingDataStream};
use crate::error::Result;

/// Factory function for creating new data stream instances.
///
/// Used by the connection pool to create new connections on demand.
pub type StreamFactory = Arc<dyn Fn() -> Box<dyn MarketDataStream> + Send + Sync>;

// ---------------------------------------------------------------------------
// Connection state
// ---------------------------------------------------------------------------

/// Tracks the lifecycle of a single pooled connection.
struct ConnectionState {
    /// Unique ID for logging and identification.
    id: u64,
    /// Tokens this connection is responsible for.
    tokens: Vec<TokenId>,
    /// When this connection was spawned.
    spawned_at: Instant,
    /// Epoch millis of the last received event (updated atomically by the task).
    last_event_at: Arc<AtomicU64>,
    /// Handle to the connection's tokio task.
    handle: tokio::task::JoinHandle<()>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns the current time as epoch milliseconds.
fn epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Spawn a connection task that reads events and forwards them to `event_tx`.
///
/// This is a free function (not a method) so both the pool and the management
/// task can call it without borrowing `self`.
fn spawn_connection(
    factory: &StreamFactory,
    reconnection_config: ReconnectionConfig,
    tokens: Vec<TokenId>,
    event_tx: mpsc::UnboundedSender<MarketEvent>,
    connection_id: u64,
    last_event_at: Arc<AtomicU64>,
) -> tokio::task::JoinHandle<()> {
    let stream = factory();
    let mut stream = ReconnectingDataStream::new(stream, reconnection_config);

    tokio::spawn(async move {
        let token_count = tokens.len();
        debug!(connection_id, tokens = token_count, "Connection task starting");

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
                    if event_tx.send(event).is_err() {
                        debug!(connection_id, "Channel closed, terminating");
                        break;
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

/// Build a fresh `ConnectionState`, spawning its task immediately.
fn new_connection(
    id: u64,
    tokens: Vec<TokenId>,
    factory: &StreamFactory,
    reconnection_config: ReconnectionConfig,
    event_tx: mpsc::UnboundedSender<MarketEvent>,
) -> ConnectionState {
    let last_event_at = Arc::new(AtomicU64::new(0));
    let handle = spawn_connection(
        factory,
        reconnection_config,
        tokens.clone(),
        event_tx,
        id,
        last_event_at.clone(),
    );
    ConnectionState {
        id,
        tokens,
        spawned_at: Instant::now(),
        last_event_at,
        handle,
    }
}

// ---------------------------------------------------------------------------
// Management task
// ---------------------------------------------------------------------------

/// Background task that monitors connection health and performs rotations.
///
/// Checks are performed every `health_check_interval_secs`. A connection is
/// replaced when any of the following is true:
///
/// 1. Its age exceeds `connection_ttl_secs - preemptive_reconnect_secs`
/// 2. It has received no events for longer than `max_silent_secs`
/// 3. Its task handle has completed (unexpected crash)
async fn management_task(
    connections: Arc<Mutex<Vec<ConnectionState>>>,
    config: ConnectionPoolConfig,
    reconnection_config: ReconnectionConfig,
    factory: StreamFactory,
    event_tx: mpsc::UnboundedSender<MarketEvent>,
    exchange_name: &'static str,
) {
    let check_interval = Duration::from_secs(config.health_check_interval_secs);
    let ttl_threshold =
        Duration::from_secs(config.connection_ttl_secs.saturating_sub(config.preemptive_reconnect_secs));
    let max_silent_ms = config.max_silent_secs * 1000;

    let mut interval = tokio::time::interval(check_interval);
    let mut next_id: u64 = 1_000_000; // well above initial IDs

    debug!(exchange = exchange_name, "Management task started");

    loop {
        interval.tick().await;
        let now = Instant::now();
        let now_ms = epoch_millis();

        // Phase 1: identify indices that need replacement (lock held briefly, no spawning).
        let indices: Vec<usize> = {
            let conns = connections.lock().unwrap();
            conns
                .iter()
                .enumerate()
                .filter_map(|(i, c)| {
                    // Crashed task
                    if c.handle.is_finished() {
                        warn!(connection_id = c.id, "Task finished unexpectedly");
                        return Some(i);
                    }
                    // TTL expiry (preemptive)
                    if now.duration_since(c.spawned_at) >= ttl_threshold {
                        info!(connection_id = c.id, age_secs = now.duration_since(c.spawned_at).as_secs(), "Approaching TTL");
                        return Some(i);
                    }
                    // Silent death
                    let last = c.last_event_at.load(Ordering::Relaxed);
                    if last > 0 && now_ms.saturating_sub(last) > max_silent_ms {
                        warn!(connection_id = c.id, silent_secs = now_ms.saturating_sub(last) / 1000, "No events, appears dead");
                        return Some(i);
                    }
                    None
                })
                .collect()
        };

        if indices.is_empty() {
            continue;
        }

        // Phase 2: spawn replacements (no lock held during spawning).
        let replacements: Vec<(usize, ConnectionState)> = indices
            .into_iter()
            .map(|i| {
                next_id += 1;
                let tokens = {
                    let conns = connections.lock().unwrap();
                    conns[i].tokens.clone()
                };
                let state = new_connection(
                    next_id,
                    tokens,
                    &factory,
                    reconnection_config.clone(),
                    event_tx.clone(),
                );
                info!(old_index = i, new_connection_id = next_id, tokens = state.tokens.len(), "Replacement spawned");
                (i, state)
            })
            .collect();

        // Phase 3: swap in replacements (brief lock, abort old tasks).
        {
            let mut conns = connections.lock().unwrap();
            for (i, new_state) in replacements {
                if i < conns.len() {
                    let old_id = conns[i].id;
                    conns[i].handle.abort();
                    conns[i] = new_state;
                    debug!(old_connection_id = old_id, new_connection_id = conns[i].id, "Swapped");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ConnectionPool
// ---------------------------------------------------------------------------

/// Exchange-agnostic connection pool that manages multiple WebSocket connections.
///
/// Distributes subscriptions across connections and provides a single merged
/// event stream. Connections are automatically rotated on TTL expiry and
/// restarted if they go silent or crash.
pub struct ConnectionPool {
    pool_config: ConnectionPoolConfig,
    reconnection_config: ReconnectionConfig,
    stream_factory: StreamFactory,
    event_rx: mpsc::UnboundedReceiver<MarketEvent>,
    event_tx: mpsc::UnboundedSender<MarketEvent>,
    connections: Arc<Mutex<Vec<ConnectionState>>>,
    management_handle: Option<tokio::task::JoinHandle<()>>,
    exchange_name: &'static str,
    next_conn_id: u64,
}

impl ConnectionPool {
    /// Create a new connection pool.
    ///
    /// No connections are opened until [`subscribe`](MarketDataStream::subscribe)
    /// is called.
    #[must_use]
    pub fn new(
        pool_config: ConnectionPoolConfig,
        reconnection_config: ReconnectionConfig,
        stream_factory: StreamFactory,
        exchange_name: &'static str,
    ) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        Self {
            pool_config,
            reconnection_config,
            stream_factory,
            event_rx,
            event_tx,
            connections: Arc::new(Mutex::new(Vec::new())),
            management_handle: None,
            exchange_name,
            next_conn_id: 0,
        }
    }

    /// Distribute `token_ids` into chunks respecting pool limits.
    ///
    /// Returns a `Vec<Vec<TokenId>>` where each inner vec is a connection's
    /// assigned tokens. Overflow tokens are appended to the last chunk.
    fn distribute_tokens(&self, token_ids: &[TokenId]) -> Vec<Vec<TokenId>> {
        let per_conn = self.pool_config.subscriptions_per_connection;
        let max_conns = self.pool_config.max_connections;
        let needed = token_ids.len().div_ceil(per_conn).min(max_conns);

        let mut chunks: Vec<Vec<TokenId>> = token_ids
            .chunks(per_conn)
            .take(needed)
            .map(|c| c.to_vec())
            .collect();

        // Overflow: tokens beyond (needed * per_conn) go to last chunk
        let assigned: usize = chunks.iter().map(Vec::len).sum();
        if assigned < token_ids.len() {
            if let Some(last) = chunks.last_mut() {
                last.extend_from_slice(&token_ids[assigned..]);
            }
        }

        chunks
    }
}

#[async_trait]
impl MarketDataStream for ConnectionPool {
    async fn connect(&mut self) -> Result<()> {
        debug!("Connection pool connect (no-op — connections created on subscribe)");
        Ok(())
    }

    async fn subscribe(&mut self, token_ids: &[TokenId]) -> Result<()> {
        if token_ids.is_empty() {
            info!("No tokens to subscribe — pool remains empty");
            return Ok(());
        }

        let chunks = self.distribute_tokens(token_ids);

        info!(
            exchange = self.exchange_name,
            tokens = token_ids.len(),
            connections = chunks.len(),
            per_conn = self.pool_config.subscriptions_per_connection,
            max_connections = self.pool_config.max_connections,
            "Creating connection pool"
        );

        // Spawn connection tasks
        let mut states = Vec::with_capacity(chunks.len());
        for (i, tokens) in chunks.into_iter().enumerate() {
            self.next_conn_id += 1;
            let id = self.next_conn_id;
            info!(connection = i + 1, connection_id = id, tokens = tokens.len(), "Spawning");

            states.push(new_connection(
                id,
                tokens,
                &self.stream_factory,
                self.reconnection_config.clone(),
                self.event_tx.clone(),
            ));
        }

        *self.connections.lock().unwrap() = states;

        // Start management task
        self.management_handle = Some(tokio::spawn(management_task(
            self.connections.clone(),
            self.pool_config.clone(),
            self.reconnection_config.clone(),
            self.stream_factory.clone(),
            self.event_tx.clone(),
            self.exchange_name,
        )));

        Ok(())
    }

    async fn next_event(&mut self) -> Option<MarketEvent> {
        self.event_rx.recv().await
    }

    fn exchange_name(&self) -> &'static str {
        self.exchange_name
    }
}

impl Drop for ConnectionPool {
    fn drop(&mut self) {
        if let Some(h) = &self.management_handle {
            h.abort();
        }
        if let Ok(conns) = self.connections.try_lock() {
            for c in conns.iter() {
                c.handle.abort();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::atomic::AtomicU32;
    use std::sync::Mutex as StdMutex;

    use crate::core::domain::OrderBook;

    // -- Mock stream ---------------------------------------------------------

    struct MockDataStream {
        events: Arc<StdMutex<VecDeque<Option<MarketEvent>>>>,
        cycle_source: Arc<StdMutex<Vec<Option<MarketEvent>>>>,
        cycle: bool,
        connect_count: Arc<AtomicU32>,
        subscribe_count: Arc<AtomicU32>,
    }

    impl MockDataStream {
        fn new() -> Self {
            Self {
                events: Arc::new(StdMutex::new(VecDeque::new())),
                cycle_source: Arc::new(StdMutex::new(Vec::new())),
                cycle: false,
                connect_count: Arc::new(AtomicU32::new(0)),
                subscribe_count: Arc::new(AtomicU32::new(0)),
            }
        }

        fn with_events(self, events: Vec<Option<MarketEvent>>) -> Self {
            *self.events.lock().unwrap() = events.clone().into();
            *self.cycle_source.lock().unwrap() = events;
            self
        }

        fn with_cycle(mut self, events: Vec<Option<MarketEvent>>) -> Self {
            *self.events.lock().unwrap() = events.clone().into();
            *self.cycle_source.lock().unwrap() = events;
            self.cycle = true;
            self
        }
    }

    #[async_trait]
    impl MarketDataStream for MockDataStream {
        async fn connect(&mut self) -> Result<()> {
            self.connect_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn subscribe(&mut self, _token_ids: &[TokenId]) -> Result<()> {
            self.subscribe_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn next_event(&mut self) -> Option<MarketEvent> {
            let event = {
                let mut q = self.events.lock().unwrap();
                let ev = q.pop_front().flatten();
                if self.cycle && q.is_empty() {
                    let src = self.cycle_source.lock().unwrap();
                    *q = src.clone().into();
                }
                ev
            };
            if event.is_some() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            event
        }

        fn exchange_name(&self) -> &'static str {
            "mock"
        }
    }

    // -- Helpers --------------------------------------------------------------

    fn pool_config(max_connections: usize, subs_per_conn: usize) -> ConnectionPoolConfig {
        ConnectionPoolConfig {
            max_connections,
            subscriptions_per_connection: subs_per_conn,
            connection_ttl_secs: 120,
            preemptive_reconnect_secs: 30,
            health_check_interval_secs: 30,
            max_silent_secs: 60,
        }
    }

    fn reconnect_config() -> ReconnectionConfig {
        ReconnectionConfig {
            initial_delay_ms: 10,
            max_delay_ms: 100,
            backoff_multiplier: 2.0,
            max_consecutive_failures: 3,
            circuit_breaker_cooldown_ms: 50,
        }
    }

    fn snapshot_event(token: &str) -> Option<MarketEvent> {
        Some(MarketEvent::OrderBookSnapshot {
            token_id: TokenId::from(token.to_string()),
            book: OrderBook::new(TokenId::from(token.to_string())),
        })
    }

    fn make_tokens(n: usize) -> Vec<TokenId> {
        (0..n).map(|i| TokenId::from(format!("t{i}"))).collect()
    }

    /// Build a factory where every spawned mock shares the same connect counter.
    fn counting_factory(
        connect_count: Arc<AtomicU32>,
        events: Vec<Option<MarketEvent>>,
        cycle: bool,
    ) -> StreamFactory {
        Arc::new(move || {
            let mut m = MockDataStream::new();
            m.connect_count = Arc::clone(&connect_count);
            let m = if cycle {
                m.with_cycle(events.clone())
            } else {
                m.with_events(events.clone())
            };
            Box::new(m)
        })
    }

    // -- Distribution tests ---------------------------------------------------

    #[tokio::test]
    async fn test_pool_single_connection() {
        let cc = Arc::new(AtomicU32::new(0));
        let factory = counting_factory(cc, vec![], false);
        let mut pool = ConnectionPool::new(pool_config(10, 500), reconnect_config(), factory, "test");

        pool.subscribe(&make_tokens(10)).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let conns = pool.connections.lock().unwrap();
        assert_eq!(conns.len(), 1);
        assert_eq!(conns[0].tokens.len(), 10);
    }

    #[tokio::test]
    async fn test_pool_multiple_connections() {
        let cc = Arc::new(AtomicU32::new(0));
        let factory = counting_factory(cc, vec![], false);
        let mut pool = ConnectionPool::new(pool_config(10, 500), reconnect_config(), factory, "test");

        pool.subscribe(&make_tokens(1000)).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let conns = pool.connections.lock().unwrap();
        assert_eq!(conns.len(), 2);
        assert_eq!(conns[0].tokens.len(), 500);
        assert_eq!(conns[1].tokens.len(), 500);
    }

    #[tokio::test]
    async fn test_pool_respects_max_connections() {
        let cc = Arc::new(AtomicU32::new(0));
        let factory = counting_factory(cc, vec![], false);
        let mut pool = ConnectionPool::new(pool_config(3, 500), reconnect_config(), factory, "test");

        pool.subscribe(&make_tokens(5000)).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let conns = pool.connections.lock().unwrap();
        assert_eq!(conns.len(), 3);
        let total: usize = conns.iter().map(|c| c.tokens.len()).sum();
        assert_eq!(total, 5000);
    }

    #[tokio::test]
    async fn test_pool_distributes_evenly() {
        let cc = Arc::new(AtomicU32::new(0));
        let factory = counting_factory(cc, vec![], false);
        let mut pool = ConnectionPool::new(pool_config(10, 500), reconnect_config(), factory, "test");

        pool.subscribe(&make_tokens(1250)).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let conns = pool.connections.lock().unwrap();
        assert_eq!(conns.len(), 3);
        assert_eq!(conns[0].tokens.len(), 500);
        assert_eq!(conns[1].tokens.len(), 500);
        assert_eq!(conns[2].tokens.len(), 250);
    }

    // -- Event merging --------------------------------------------------------

    #[tokio::test]
    async fn test_pool_merges_events() {
        let events = vec![snapshot_event("t1"), snapshot_event("t2")];
        let cc = Arc::new(AtomicU32::new(0));
        let factory = counting_factory(cc, events, false);
        let mut pool = ConnectionPool::new(pool_config(10, 500), reconnect_config(), factory, "test");

        pool.subscribe(&make_tokens(1)).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert!(matches!(pool.next_event().await, Some(MarketEvent::OrderBookSnapshot { .. })));
        assert!(matches!(pool.next_event().await, Some(MarketEvent::OrderBookSnapshot { .. })));
    }

    // -- Identity -------------------------------------------------------------

    #[tokio::test]
    async fn test_pool_exchange_name() {
        let factory: StreamFactory = Arc::new(|| Box::new(MockDataStream::new()));
        let cfg = pool_config(10, 500);

        let p1 = ConnectionPool::new(cfg.clone(), reconnect_config(), factory.clone(), "polymarket");
        assert_eq!(p1.exchange_name(), "polymarket");

        let p2 = ConnectionPool::new(cfg, reconnect_config(), factory, "kalshi");
        assert_eq!(p2.exchange_name(), "kalshi");
    }

    // -- Edge cases -----------------------------------------------------------

    #[tokio::test]
    async fn test_pool_connect_is_noop() {
        let factory: StreamFactory = Arc::new(|| Box::new(MockDataStream::new()));
        let mut pool = ConnectionPool::new(pool_config(10, 500), reconnect_config(), factory, "test");

        assert!(pool.connect().await.is_ok());
        assert!(pool.connections.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_pool_empty_subscribe() {
        let factory: StreamFactory = Arc::new(|| Box::new(MockDataStream::new()));
        let mut pool = ConnectionPool::new(pool_config(10, 500), reconnect_config(), factory, "test");

        assert!(pool.subscribe(&[]).await.is_ok());
        assert!(pool.connections.lock().unwrap().is_empty());
    }

    // -- Health monitoring ----------------------------------------------------

    #[tokio::test]
    async fn test_pool_ttl_rotation() {
        let cc = Arc::new(AtomicU32::new(0));
        let factory = counting_factory(cc.clone(), vec![snapshot_event("t1")], true);

        let mut cfg = pool_config(10, 500);
        cfg.connection_ttl_secs = 1;
        cfg.preemptive_reconnect_secs = 0;
        cfg.health_check_interval_secs = 1;

        let mut pool = ConnectionPool::new(cfg, reconnect_config(), factory, "test");
        pool.subscribe(&make_tokens(1)).await.unwrap();

        tokio::time::sleep(Duration::from_secs(3)).await;
        assert!(cc.load(Ordering::SeqCst) > 1, "Expected TTL rotation");
    }

    #[tokio::test]
    async fn test_pool_preemptive_reconnect() {
        let cc = Arc::new(AtomicU32::new(0));
        let factory = counting_factory(cc.clone(), vec![snapshot_event("t1")], true);

        let mut cfg = pool_config(10, 500);
        cfg.connection_ttl_secs = 3;
        cfg.preemptive_reconnect_secs = 2; // threshold = 3 - 2 = 1s
        cfg.health_check_interval_secs = 1;

        let mut pool = ConnectionPool::new(cfg, reconnect_config(), factory, "test");
        pool.subscribe(&make_tokens(1)).await.unwrap();

        tokio::time::sleep(Duration::from_secs(2)).await;
        assert!(cc.load(Ordering::SeqCst) > 1, "Expected preemptive reconnect at ~1s");
    }

    #[tokio::test]
    async fn test_pool_silent_death_detection() {
        let cc = Arc::new(AtomicU32::new(0));
        // Sends 1 event then stops — goes silent
        let factory = counting_factory(cc.clone(), vec![snapshot_event("t1")], false);

        let mut cfg = pool_config(10, 500);
        cfg.max_silent_secs = 1;
        cfg.health_check_interval_secs = 1;
        cfg.connection_ttl_secs = 120;

        let mut pool = ConnectionPool::new(cfg, reconnect_config(), factory, "test");
        pool.subscribe(&make_tokens(1)).await.unwrap();

        tokio::time::sleep(Duration::from_secs(4)).await;
        assert!(cc.load(Ordering::SeqCst) > 1, "Expected restart after silence");
    }

    #[tokio::test]
    async fn test_pool_crashed_task_restart() {
        let cc = Arc::new(AtomicU32::new(0));
        // Empty events = stream returns None immediately = task finishes
        let factory = counting_factory(cc.clone(), vec![], false);

        let mut cfg = pool_config(10, 500);
        cfg.health_check_interval_secs = 1;

        let mut pool = ConnectionPool::new(cfg, reconnect_config(), factory, "test");
        pool.subscribe(&make_tokens(1)).await.unwrap();

        tokio::time::sleep(Duration::from_secs(3)).await;
        assert!(cc.load(Ordering::SeqCst) > 1, "Expected crashed task restart");
    }

    #[tokio::test]
    async fn test_pool_healthy_connection_not_replaced() {
        let cc = Arc::new(AtomicU32::new(0));
        let factory = counting_factory(cc.clone(), vec![snapshot_event("t1")], true);

        let mut cfg = pool_config(10, 500);
        cfg.connection_ttl_secs = 120;
        cfg.max_silent_secs = 60;
        cfg.health_check_interval_secs = 1;

        let mut pool = ConnectionPool::new(cfg, reconnect_config(), factory, "test");
        pool.subscribe(&make_tokens(1)).await.unwrap();

        tokio::time::sleep(Duration::from_secs(3)).await;
        assert_eq!(cc.load(Ordering::SeqCst), 1, "Healthy connection should not be replaced");
    }
}
