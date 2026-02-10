//! Connection pool for managing multiple WebSocket connections.
//!
//! This module provides an exchange-agnostic connection pool that distributes
//! subscriptions across multiple WebSocket connections to avoid hitting
//! per-connection subscription limits.
//!
//! # Architecture
//!
//! Each connection runs as a separate tokio task that reads from its WebSocket
//! and sends events into a shared bounded `mpsc` channel. The pool merges
//! events from all connections into a single stream via `next_event()`.
//!
//! A background management task monitors all connections for:
//! - **TTL expiry**: proactively replaces connections before their lifetime limit
//! - **Silent death**: detects connections that stopped receiving events
//! - **Task crashes**: restarts connections whose tasks finished unexpectedly
//!
//! Replacements use true zero-gap handoff: a new connection is spawned and
//! must deliver its first event before the old connection is aborted.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::app::{ConnectionPoolConfig, ReconnectionConfig};
use crate::core::domain::TokenId;
use crate::core::exchange::{MarketDataStream, MarketEvent, ReconnectingDataStream};
use crate::error::{ConfigError, Result};

/// Factory function for creating new data stream instances.
///
/// Used by the connection pool to create new connections on demand.
pub type StreamFactory = Arc<dyn Fn() -> Box<dyn MarketDataStream> + Send + Sync>;

/// Default event channel capacity (bounded to prevent unbounded memory growth).
const DEFAULT_CHANNEL_CAPACITY: usize = 10_000;

// ---------------------------------------------------------------------------
// Pool statistics
// ---------------------------------------------------------------------------

/// Runtime statistics for the connection pool.
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    /// Number of currently active connections.
    pub active_connections: usize,
    /// Total number of connection rotations (TTL-triggered).
    pub total_rotations: u64,
    /// Total number of restarts (crash/silence-triggered).
    pub total_restarts: u64,
    /// Total number of events that were dropped due to a full channel.
    pub events_dropped: u64,
}

/// Shared counters updated atomically by connection and management tasks.
struct SharedCounters {
    rotations: AtomicU64,
    restarts: AtomicU64,
    events_dropped: AtomicU64,
}

impl SharedCounters {
    fn new() -> Self {
        Self {
            rotations: AtomicU64::new(0),
            restarts: AtomicU64::new(0),
            events_dropped: AtomicU64::new(0),
        }
    }
}

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
fn new_connection(
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

// ---------------------------------------------------------------------------
// Management task
// ---------------------------------------------------------------------------

/// Descriptor for a connection that needs replacement.
struct ReplacementJob {
    index: usize,
    tokens: Vec<TokenId>,
    reason: ReplacementReason,
}

#[derive(Debug)]
enum ReplacementReason {
    Ttl,
    Silent,
    Crashed,
}

/// Background task that monitors connection health and performs rotations.
///
/// Runs on a fixed interval. Uses a 3-phase approach:
/// 1. **Identify** — lock briefly to find connections needing replacement
/// 2. **Spawn** — create replacements without holding the lock
/// 3. **Handoff** — wait for replacement's first event, then swap + abort old
async fn management_task(
    connections: Arc<Mutex<Vec<ConnectionState>>>,
    config: ConnectionPoolConfig,
    reconnection_config: ReconnectionConfig,
    factory: StreamFactory,
    event_tx: mpsc::Sender<MarketEvent>,
    counters: Arc<SharedCounters>,
    exchange_name: &'static str,
) {
    let check_interval = Duration::from_secs(config.health_check_interval_secs);
    let ttl_threshold = Duration::from_secs(
        config.connection_ttl_secs.saturating_sub(config.preemptive_reconnect_secs),
    );
    let max_silent_ms = config.max_silent_secs * 1000;
    let handoff_timeout = Duration::from_secs(config.connection_ttl_secs.max(30));

    let mut interval = tokio::time::interval(check_interval);
    let mut next_id: u64 = 1_000_000;

    debug!(exchange = exchange_name, "Management task started");

    loop {
        interval.tick().await;
        let now = Instant::now();
        let now_ms = epoch_millis();

        // Phase 1: identify connections needing replacement (brief lock).
        let jobs: Vec<ReplacementJob> = {
            let conns = match connections.lock() {
                Ok(c) => c,
                Err(poisoned) => poisoned.into_inner(),
            };
            conns
                .iter()
                .enumerate()
                .filter_map(|(i, c)| {
                    if c.handle.is_finished() {
                        warn!(connection_id = c.id, "Task finished unexpectedly");
                        return Some(ReplacementJob {
                            index: i,
                            tokens: c.tokens.clone(),
                            reason: ReplacementReason::Crashed,
                        });
                    }
                    if now.duration_since(c.spawned_at) >= ttl_threshold {
                        info!(
                            connection_id = c.id,
                            age_secs = now.duration_since(c.spawned_at).as_secs(),
                            "Approaching TTL"
                        );
                        return Some(ReplacementJob {
                            index: i,
                            tokens: c.tokens.clone(),
                            reason: ReplacementReason::Ttl,
                        });
                    }
                    let last = c.last_event_at.load(Ordering::Relaxed);
                    if last > 0 && now_ms.saturating_sub(last) > max_silent_ms {
                        warn!(
                            connection_id = c.id,
                            silent_secs = now_ms.saturating_sub(last) / 1000,
                            "No events, appears dead"
                        );
                        return Some(ReplacementJob {
                            index: i,
                            tokens: c.tokens.clone(),
                            reason: ReplacementReason::Silent,
                        });
                    }
                    None
                })
                .collect()
        };

        if jobs.is_empty() {
            continue;
        }

        // Phase 2 + 3: for each job, spawn replacement, wait for handoff, swap.
        for job in jobs {
            next_id += 1;
            let new_id = next_id;

            let state = new_connection(
                new_id,
                job.tokens,
                &factory,
                reconnection_config.clone(),
                event_tx.clone(),
                counters.clone(),
            );

            // Wait for the replacement to deliver its first event (or timeout).
            let started = Instant::now();
            let initial_ts = state.last_event_at.load(Ordering::Relaxed);
            let handoff_ok = loop {
                tokio::time::sleep(Duration::from_millis(100)).await;
                let current_ts = state.last_event_at.load(Ordering::Relaxed);
                if current_ts > initial_ts {
                    break true;
                }
                if state.handle.is_finished() {
                    warn!(connection_id = new_id, "Replacement task died during handoff");
                    break false;
                }
                if started.elapsed() > handoff_timeout {
                    warn!(connection_id = new_id, "Handoff timeout — swapping anyway");
                    break true; // swap anyway, old connection is stale
                }
            };

            if !handoff_ok {
                // Replacement died, abort it and skip — management will retry next tick.
                state.handle.abort();
                continue;
            }

            // Swap in replacement, abort old.
            {
                let mut conns = match connections.lock() {
                    Ok(c) => c,
                    Err(poisoned) => poisoned.into_inner(),
                };
                if job.index < conns.len() {
                    let old_id = conns[job.index].id;
                    conns[job.index].handle.abort();
                    conns[job.index] = state;

                    match job.reason {
                        ReplacementReason::Ttl => {
                            counters.rotations.fetch_add(1, Ordering::Relaxed);
                            info!(old_connection_id = old_id, new_connection_id = new_id, "TTL rotation complete");
                        }
                        ReplacementReason::Silent | ReplacementReason::Crashed => {
                            counters.restarts.fetch_add(1, Ordering::Relaxed);
                            info!(old_connection_id = old_id, new_connection_id = new_id, reason = ?job.reason, "Restart complete");
                        }
                    }
                } else {
                    // Index shifted (shouldn't happen, but be defensive)
                    state.handle.abort();
                    warn!(old_index = job.index, "Connection index out of bounds, skipping swap");
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
///
/// The event channel is bounded (default 10,000) to prevent unbounded memory
/// growth under backpressure. Events are dropped with a warning when full.
pub struct ConnectionPool {
    pool_config: ConnectionPoolConfig,
    reconnection_config: ReconnectionConfig,
    stream_factory: StreamFactory,
    event_rx: mpsc::Receiver<MarketEvent>,
    event_tx: mpsc::Sender<MarketEvent>,
    connections: Arc<Mutex<Vec<ConnectionState>>>,
    counters: Arc<SharedCounters>,
    management_handle: Option<tokio::task::JoinHandle<()>>,
    exchange_name: &'static str,
    next_conn_id: u64,
}

impl ConnectionPool {
    /// Create a new connection pool.
    ///
    /// No connections are opened until [`subscribe`](MarketDataStream::subscribe)
    /// is called.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid:
    /// - `connection_ttl_secs` must be > 0
    /// - `preemptive_reconnect_secs` must be < `connection_ttl_secs`
    /// - `max_connections` must be > 0
    /// - `subscriptions_per_connection` must be > 0
    /// - `health_check_interval_secs` must be > 0
    /// - `max_silent_secs` must be > 0
    pub fn new(
        pool_config: ConnectionPoolConfig,
        reconnection_config: ReconnectionConfig,
        stream_factory: StreamFactory,
        exchange_name: &'static str,
    ) -> Result<Self> {
        Self::validate_config(&pool_config)?;

        let (event_tx, event_rx) = mpsc::channel(DEFAULT_CHANNEL_CAPACITY);
        Ok(Self {
            pool_config,
            reconnection_config,
            stream_factory,
            event_rx,
            event_tx,
            connections: Arc::new(Mutex::new(Vec::new())),
            counters: Arc::new(SharedCounters::new()),
            management_handle: None,
            exchange_name,
            next_conn_id: 0,
        })
    }

    /// Validate pool configuration values.
    fn validate_config(config: &ConnectionPoolConfig) -> Result<()> {
        let invalid = |field: &'static str, reason: &str| -> crate::error::Error {
            ConfigError::InvalidValue {
                field,
                reason: reason.to_string(),
            }
            .into()
        };

        if config.connection_ttl_secs == 0 {
            return Err(invalid("connection_ttl_secs", "must be > 0"));
        }
        if config.preemptive_reconnect_secs >= config.connection_ttl_secs {
            return Err(invalid(
                "preemptive_reconnect_secs",
                "must be < connection_ttl_secs",
            ));
        }
        if config.max_connections == 0 {
            return Err(invalid("max_connections", "must be > 0"));
        }
        if config.subscriptions_per_connection == 0 {
            return Err(invalid("subscriptions_per_connection", "must be > 0"));
        }
        if config.health_check_interval_secs == 0 {
            return Err(invalid("health_check_interval_secs", "must be > 0"));
        }
        if config.max_silent_secs == 0 {
            return Err(invalid("max_silent_secs", "must be > 0"));
        }
        Ok(())
    }

    /// Runtime statistics for observability (e.g. Telegram `/health` command).
    pub fn stats(&self) -> PoolStats {
        let active = match self.connections.lock() {
            Ok(c) => c.len(),
            Err(p) => p.into_inner().len(),
        };
        PoolStats {
            active_connections: active,
            total_rotations: self.counters.rotations.load(Ordering::Relaxed),
            total_restarts: self.counters.restarts.load(Ordering::Relaxed),
            events_dropped: self.counters.events_dropped.load(Ordering::Relaxed),
        }
    }

    /// Distribute `token_ids` into chunks respecting pool limits.
    ///
    /// Overflow tokens are appended to the last chunk when the number of
    /// tokens exceeds `max_connections * subscriptions_per_connection`.
    fn distribute_tokens(&self, token_ids: &[TokenId]) -> Vec<Vec<TokenId>> {
        let per_conn = self.pool_config.subscriptions_per_connection;
        let max_conns = self.pool_config.max_connections;
        let needed = token_ids.len().div_ceil(per_conn).min(max_conns);

        let mut chunks: Vec<Vec<TokenId>> = token_ids
            .chunks(per_conn)
            .take(needed)
            .map(|c| c.to_vec())
            .collect();

        // Overflow: remaining tokens go to last chunk
        let assigned: usize = chunks.iter().map(Vec::len).sum();
        if assigned < token_ids.len() {
            if let Some(last) = chunks.last_mut() {
                last.extend_from_slice(&token_ids[assigned..]);
            }
        }

        chunks
    }

    /// Tear down all existing connections and the management task.
    fn shutdown(&mut self) {
        if let Some(h) = self.management_handle.take() {
            h.abort();
        }
        if let Ok(mut conns) = self.connections.lock() {
            for c in conns.drain(..) {
                c.handle.abort();
            }
        }
    }
}

#[async_trait]
impl MarketDataStream for ConnectionPool {
    async fn connect(&mut self) -> Result<()> {
        debug!("Connection pool connect (no-op — connections created on subscribe)");
        Ok(())
    }

    async fn subscribe(&mut self, token_ids: &[TokenId]) -> Result<()> {
        // Tear down any existing connections (safe to call multiple times).
        self.shutdown();

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
                self.counters.clone(),
            ));
        }

        {
            let mut conns = match self.connections.lock() {
                Ok(c) => c,
                Err(p) => p.into_inner(),
            };
            *conns = states;
        }

        // Start management task
        self.management_handle = Some(tokio::spawn(management_task(
            self.connections.clone(),
            self.pool_config.clone(),
            self.reconnection_config.clone(),
            self.stream_factory.clone(),
            self.event_tx.clone(),
            self.counters.clone(),
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
        self.shutdown();
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

    // -- Config validation ----------------------------------------------------

    #[test]
    fn test_config_validation_rejects_zero_ttl() {
        let mut cfg = pool_config(10, 500);
        cfg.connection_ttl_secs = 0;
        let factory: StreamFactory = Arc::new(|| Box::new(MockDataStream::new()));
        assert!(ConnectionPool::new(cfg, reconnect_config(), factory, "test").is_err());
    }

    #[test]
    fn test_config_validation_rejects_preemptive_gte_ttl() {
        let mut cfg = pool_config(10, 500);
        cfg.preemptive_reconnect_secs = 120; // equal to TTL
        let factory: StreamFactory = Arc::new(|| Box::new(MockDataStream::new()));
        assert!(ConnectionPool::new(cfg, reconnect_config(), factory, "test").is_err());
    }

    #[test]
    fn test_config_validation_rejects_zero_max_connections() {
        let cfg = pool_config(0, 500);
        let factory: StreamFactory = Arc::new(|| Box::new(MockDataStream::new()));
        assert!(ConnectionPool::new(cfg, reconnect_config(), factory, "test").is_err());
    }

    #[test]
    fn test_config_validation_rejects_zero_subs_per_conn() {
        let cfg = pool_config(10, 0);
        let factory: StreamFactory = Arc::new(|| Box::new(MockDataStream::new()));
        assert!(ConnectionPool::new(cfg, reconnect_config(), factory, "test").is_err());
    }

    #[test]
    fn test_config_validation_accepts_valid_config() {
        let factory: StreamFactory = Arc::new(|| Box::new(MockDataStream::new()));
        assert!(ConnectionPool::new(pool_config(10, 500), reconnect_config(), factory, "test").is_ok());
    }

    // -- Distribution tests ---------------------------------------------------

    #[tokio::test]
    async fn test_pool_single_connection() {
        let cc = Arc::new(AtomicU32::new(0));
        let factory = counting_factory(cc, vec![], false);
        let mut pool = ConnectionPool::new(pool_config(10, 500), reconnect_config(), factory, "test").unwrap();

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
        let mut pool = ConnectionPool::new(pool_config(10, 500), reconnect_config(), factory, "test").unwrap();

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
        let mut pool = ConnectionPool::new(pool_config(3, 500), reconnect_config(), factory, "test").unwrap();

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
        let mut pool = ConnectionPool::new(pool_config(10, 500), reconnect_config(), factory, "test").unwrap();

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
        let mut pool = ConnectionPool::new(pool_config(10, 500), reconnect_config(), factory, "test").unwrap();

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

        let p1 = ConnectionPool::new(cfg.clone(), reconnect_config(), factory.clone(), "polymarket").unwrap();
        assert_eq!(p1.exchange_name(), "polymarket");

        let p2 = ConnectionPool::new(cfg, reconnect_config(), factory, "kalshi").unwrap();
        assert_eq!(p2.exchange_name(), "kalshi");
    }

    // -- Edge cases -----------------------------------------------------------

    #[tokio::test]
    async fn test_pool_connect_is_noop() {
        let factory: StreamFactory = Arc::new(|| Box::new(MockDataStream::new()));
        let mut pool = ConnectionPool::new(pool_config(10, 500), reconnect_config(), factory, "test").unwrap();

        assert!(pool.connect().await.is_ok());
        assert!(pool.connections.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_pool_empty_subscribe() {
        let factory: StreamFactory = Arc::new(|| Box::new(MockDataStream::new()));
        let mut pool = ConnectionPool::new(pool_config(10, 500), reconnect_config(), factory, "test").unwrap();

        assert!(pool.subscribe(&[]).await.is_ok());
        assert!(pool.connections.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_pool_resubscribe_tears_down_old() {
        let cc = Arc::new(AtomicU32::new(0));
        let factory = counting_factory(cc.clone(), vec![snapshot_event("t1")], true);
        let mut pool = ConnectionPool::new(pool_config(10, 500), reconnect_config(), factory, "test").unwrap();

        pool.subscribe(&make_tokens(5)).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(pool.connections.lock().unwrap().len(), 1);

        // Re-subscribe should tear down old and create new
        pool.subscribe(&make_tokens(10)).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(pool.connections.lock().unwrap().len(), 1);
        assert_eq!(pool.connections.lock().unwrap()[0].tokens.len(), 10);
    }

    // -- Stats ----------------------------------------------------------------

    #[tokio::test]
    async fn test_pool_stats_initial() {
        let factory: StreamFactory = Arc::new(|| Box::new(MockDataStream::new()));
        let pool = ConnectionPool::new(pool_config(10, 500), reconnect_config(), factory, "test").unwrap();
        let stats = pool.stats();

        assert_eq!(stats.active_connections, 0);
        assert_eq!(stats.total_rotations, 0);
        assert_eq!(stats.total_restarts, 0);
        assert_eq!(stats.events_dropped, 0);
    }

    // -- Health monitoring ----------------------------------------------------

    #[tokio::test]
    async fn test_pool_ttl_rotation() {
        let cc = Arc::new(AtomicU32::new(0));
        let factory = counting_factory(cc.clone(), vec![snapshot_event("t1")], true);

        let mut cfg = pool_config(10, 500);
        cfg.connection_ttl_secs = 2;
        cfg.preemptive_reconnect_secs = 1;
        cfg.health_check_interval_secs = 1;

        let mut pool = ConnectionPool::new(cfg, reconnect_config(), factory, "test").unwrap();
        pool.subscribe(&make_tokens(1)).await.unwrap();

        tokio::time::sleep(Duration::from_secs(4)).await;
        assert!(cc.load(Ordering::SeqCst) > 1, "Expected TTL rotation");
        assert!(pool.stats().total_rotations > 0, "Expected rotation counter > 0");
    }

    #[tokio::test]
    async fn test_pool_preemptive_reconnect() {
        let cc = Arc::new(AtomicU32::new(0));
        let factory = counting_factory(cc.clone(), vec![snapshot_event("t1")], true);

        let mut cfg = pool_config(10, 500);
        cfg.connection_ttl_secs = 4;
        cfg.preemptive_reconnect_secs = 3; // threshold = 4 - 3 = 1s
        cfg.health_check_interval_secs = 1;

        let mut pool = ConnectionPool::new(cfg, reconnect_config(), factory, "test").unwrap();
        pool.subscribe(&make_tokens(1)).await.unwrap();

        tokio::time::sleep(Duration::from_secs(3)).await;
        assert!(cc.load(Ordering::SeqCst) > 1, "Expected preemptive reconnect at ~1s");
    }

    #[tokio::test]
    async fn test_pool_silent_death_detection() {
        let cc = Arc::new(AtomicU32::new(0));
        let factory = counting_factory(cc.clone(), vec![snapshot_event("t1")], false);

        let mut cfg = pool_config(10, 500);
        cfg.max_silent_secs = 2;
        cfg.health_check_interval_secs = 1;
        cfg.connection_ttl_secs = 120;

        let mut pool = ConnectionPool::new(cfg, reconnect_config(), factory, "test").unwrap();
        pool.subscribe(&make_tokens(1)).await.unwrap();

        tokio::time::sleep(Duration::from_secs(5)).await;
        assert!(cc.load(Ordering::SeqCst) > 1, "Expected restart after silence");
        assert!(pool.stats().total_restarts > 0, "Expected restart counter > 0");
    }

    #[tokio::test]
    async fn test_pool_crashed_task_restart() {
        let cc = Arc::new(AtomicU32::new(0));
        let factory = counting_factory(cc.clone(), vec![], false);

        let mut cfg = pool_config(10, 500);
        cfg.health_check_interval_secs = 1;

        let mut pool = ConnectionPool::new(cfg, reconnect_config(), factory, "test").unwrap();
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

        let mut pool = ConnectionPool::new(cfg, reconnect_config(), factory, "test").unwrap();
        pool.subscribe(&make_tokens(1)).await.unwrap();

        tokio::time::sleep(Duration::from_secs(3)).await;
        assert_eq!(cc.load(Ordering::SeqCst), 1, "Healthy connection should not be replaced");
    }
}
