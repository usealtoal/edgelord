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
//! A management task monitors all connections for:
//! - **TTL expiry**: proactively replaces connections before they hit their lifetime limit
//! - **Silent death**: detects connections that are "connected" but not receiving events
//! - **Task crashes**: restarts connections whose tasks have unexpectedly finished
//!
//! This design avoids borrow checker issues by eliminating the need to hold
//! mutable references to connections while iterating over them.

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
/// This is used by the connection pool to create new connections on demand.
pub type StreamFactory = Arc<dyn Fn() -> Box<dyn MarketDataStream> + Send + Sync>;

/// State for a single connection in the pool.
struct ConnectionState {
    /// Unique ID for this connection.
    id: u64,
    /// Tokens this connection is responsible for.
    tokens: Vec<TokenId>,
    /// When this connection was spawned.
    spawned_at: Instant,
    /// Last time an event was received from this connection (epoch millis).
    last_event_at: Arc<AtomicU64>,
    /// Handle to the connection task.
    handle: tokio::task::JoinHandle<()>,
    /// Whether this connection is being replaced (avoid double-replacement).
    replacing: bool,
}

/// Connection pool that manages multiple WebSocket connections.
///
/// The pool distributes subscriptions across multiple connections to avoid
/// hitting per-connection limits. Each connection runs in a separate task
/// and sends events to a shared channel.
pub struct ConnectionPool {
    /// Connection pool configuration.
    pool_config: ConnectionPoolConfig,
    /// Reconnection configuration for individual connections.
    reconnection_config: ReconnectionConfig,
    /// Factory for creating new data stream instances.
    stream_factory: StreamFactory,
    /// Receive merged events from all connection tasks.
    event_rx: mpsc::UnboundedReceiver<MarketEvent>,
    /// Clone this for each new connection task.
    event_tx: mpsc::UnboundedSender<MarketEvent>,
    /// Shared state for management task.
    connections: Arc<Mutex<Vec<ConnectionState>>>,
    /// Management task handle.
    management_handle: Option<tokio::task::JoinHandle<()>>,
    /// Track subscription map for external visibility.
    subscription_map: Vec<Vec<TokenId>>,
    /// Exchange name for identification.
    exchange_name: &'static str,
    /// Counter for connection IDs.
    next_conn_id: u64,
}

/// Helper to get current epoch milliseconds.
fn current_epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Spawn a connection task for the given tokens.
///
/// Each connection task:
/// 1. Wraps the stream in a `ReconnectingDataStream`
/// 2. Connects and subscribes to its assigned tokens
/// 3. Continuously reads events and forwards them to the shared channel
/// 4. Updates `last_event_at` timestamp on each event
/// 5. Terminates when the stream ends or channel is closed
fn spawn_connection_task_static(
    stream_factory: StreamFactory,
    reconnection_config: ReconnectionConfig,
    tokens: Vec<TokenId>,
    event_tx: mpsc::UnboundedSender<MarketEvent>,
    connection_id: u64,
    last_event_at: Arc<AtomicU64>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let stream = stream_factory();
        let mut reconnecting_stream = ReconnectingDataStream::new(stream, reconnection_config);
        let token_count = tokens.len();

        debug!(
            connection_id = connection_id,
            tokens = token_count,
            "Connection task starting"
        );

        // Connect
        if let Err(e) = reconnecting_stream.connect().await {
            error!(
                connection_id = connection_id,
                error = %e,
                "Connection task failed to connect"
            );
            return;
        }

        // Subscribe
        if let Err(e) = reconnecting_stream.subscribe(&tokens).await {
            error!(
                connection_id = connection_id,
                error = %e,
                "Connection task failed to subscribe"
            );
            return;
        }

        debug!(
            connection_id = connection_id,
            tokens = token_count,
            "Connection task subscribed"
        );

        // Event loop
        loop {
            match reconnecting_stream.next_event().await {
                Some(event) => {
                    // Update last event timestamp
                    last_event_at.store(current_epoch_millis(), Ordering::Relaxed);

                    if event_tx.send(event).is_err() {
                        debug!(
                            connection_id = connection_id,
                            "Connection task: event channel closed, terminating"
                        );
                        break;
                    }
                }
                None => {
                    warn!(
                        connection_id = connection_id,
                        "Connection task: stream ended"
                    );
                    break;
                }
            }
        }

        debug!(connection_id = connection_id, "Connection task terminated");
    })
}

/// Management task that monitors connection health and performs TTL rotation.
async fn management_task(
    connections: Arc<Mutex<Vec<ConnectionState>>>,
    pool_config: ConnectionPoolConfig,
    reconnection_config: ReconnectionConfig,
    stream_factory: StreamFactory,
    event_tx: mpsc::UnboundedSender<MarketEvent>,
    exchange_name: &'static str,
) {
    let mut interval =
        tokio::time::interval(Duration::from_secs(pool_config.health_check_interval_secs));
    let mut next_conn_id: u64 = 1000; // start high to avoid collision with initial IDs

    debug!(
        exchange = exchange_name,
        check_interval_secs = pool_config.health_check_interval_secs,
        "Management task started"
    );

    loop {
        interval.tick().await;
        let now = Instant::now();
        let now_millis = current_epoch_millis();

        let mut conns = connections.lock().unwrap();
        let mut to_replace: Vec<usize> = Vec::new();

        for (i, conn) in conns.iter().enumerate() {
            if conn.replacing {
                continue;
            }

            let age = now.duration_since(conn.spawned_at);
            let ttl = Duration::from_secs(pool_config.connection_ttl_secs);
            let preemptive = Duration::from_secs(pool_config.preemptive_reconnect_secs);

            // Check TTL (preemptive)
            if age >= ttl.saturating_sub(preemptive) {
                info!(
                    connection_id = conn.id,
                    age_secs = age.as_secs(),
                    ttl_secs = pool_config.connection_ttl_secs,
                    "Connection approaching TTL, scheduling replacement"
                );
                to_replace.push(i);
                continue;
            }

            // Check silent death
            let last_event = conn.last_event_at.load(Ordering::Relaxed);
            if last_event > 0 {
                let silent_duration_ms = now_millis.saturating_sub(last_event);
                if silent_duration_ms > pool_config.max_silent_secs * 1000 {
                    warn!(
                        connection_id = conn.id,
                        silent_secs = silent_duration_ms / 1000,
                        "Connection appears dead (no events), scheduling replacement"
                    );
                    to_replace.push(i);
                }
            }
        }

        // Replace connections that need it
        for &i in to_replace.iter().rev() {
            let old_conn = &mut conns[i];
            old_conn.replacing = true;
            let tokens = old_conn.tokens.clone();
            let old_id = old_conn.id;

            next_conn_id += 1;
            let new_id = next_conn_id;

            // Spawn replacement
            let last_event_at = Arc::new(AtomicU64::new(0));
            let handle = spawn_connection_task_static(
                stream_factory.clone(),
                reconnection_config.clone(),
                tokens.clone(),
                event_tx.clone(),
                new_id,
                last_event_at.clone(),
            );

            info!(
                old_connection_id = old_id,
                new_connection_id = new_id,
                tokens = tokens.len(),
                "Replacement connection spawned"
            );

            // Abort old connection
            old_conn.handle.abort();

            // Replace in-place
            *old_conn = ConnectionState {
                id: new_id,
                tokens,
                spawned_at: Instant::now(),
                last_event_at,
                handle,
                replacing: false,
            };
        }

        // Also check for tasks that have finished unexpectedly
        for conn in conns.iter_mut() {
            if conn.handle.is_finished() && !conn.replacing {
                warn!(
                    connection_id = conn.id,
                    "Connection task finished unexpectedly, restarting"
                );
                conn.replacing = true;

                let tokens = conn.tokens.clone();
                next_conn_id += 1;
                let new_id = next_conn_id;
                let last_event_at = Arc::new(AtomicU64::new(0));

                let handle = spawn_connection_task_static(
                    stream_factory.clone(),
                    reconnection_config.clone(),
                    tokens.clone(),
                    event_tx.clone(),
                    new_id,
                    last_event_at.clone(),
                );

                info!(
                    old_connection_id = conn.id,
                    new_connection_id = new_id,
                    "Crashed connection restarted"
                );

                *conn = ConnectionState {
                    id: new_id,
                    tokens,
                    spawned_at: Instant::now(),
                    last_event_at,
                    handle,
                    replacing: false,
                };
            }
        }
    }
}

impl ConnectionPool {
    /// Create a new connection pool.
    ///
    /// # Arguments
    ///
    /// * `pool_config` - Connection pool configuration
    /// * `reconnection_config` - Reconnection settings for individual connections
    /// * `stream_factory` - Factory function for creating new data streams
    /// * `exchange_name` - Name of the exchange (e.g., "polymarket")
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
            subscription_map: Vec::new(),
            exchange_name,
            next_conn_id: 0,
        }
    }
}

#[async_trait]
impl MarketDataStream for ConnectionPool {
    async fn connect(&mut self) -> Result<()> {
        // No-op: actual connections happen in subscribe()
        debug!("Connection pool connect called (no-op)");
        Ok(())
    }

    async fn subscribe(&mut self, token_ids: &[TokenId]) -> Result<()> {
        if token_ids.is_empty() {
            info!("No tokens to subscribe, connection pool remains empty");
            return Ok(());
        }

        // Calculate number of connections needed
        let tokens_per_conn = self.pool_config.subscriptions_per_connection;
        let needed_connections = token_ids.len().div_ceil(tokens_per_conn);
        let actual_connections = needed_connections.min(self.pool_config.max_connections);

        info!(
            tokens = token_ids.len(),
            connections = actual_connections,
            tokens_per_conn = tokens_per_conn,
            max_connections = self.pool_config.max_connections,
            "Creating connection pool"
        );

        // Distribute tokens across connections
        let mut token_chunks: Vec<Vec<TokenId>> = Vec::new();
        for chunk in token_ids.chunks(tokens_per_conn) {
            token_chunks.push(chunk.to_vec());
            if token_chunks.len() >= actual_connections {
                break;
            }
        }

        // If we have more tokens than connections allow, merge remaining tokens
        // into the last connection
        if token_ids.len() > actual_connections * tokens_per_conn {
            let assigned: usize = token_chunks.iter().map(Vec::len).sum();
            if assigned < token_ids.len() {
                let remaining = &token_ids[assigned..];
                if let Some(last_chunk) = token_chunks.last_mut() {
                    last_chunk.extend_from_slice(remaining);
                }
            }
        }

        // Build connection states
        let mut conn_states = Vec::new();
        for (index, chunk) in token_chunks.iter().enumerate() {
            self.next_conn_id += 1;
            let conn_id = self.next_conn_id;

            info!(
                connection = index + 1,
                connection_id = conn_id,
                tokens = chunk.len(),
                "Spawning connection task"
            );

            let last_event_at = Arc::new(AtomicU64::new(0));
            let handle = spawn_connection_task_static(
                self.stream_factory.clone(),
                self.reconnection_config.clone(),
                chunk.clone(),
                self.event_tx.clone(),
                conn_id,
                last_event_at.clone(),
            );

            conn_states.push(ConnectionState {
                id: conn_id,
                tokens: chunk.clone(),
                spawned_at: Instant::now(),
                last_event_at,
                handle,
                replacing: false,
            });

            self.subscription_map.push(chunk.clone());
        }

        // Store connection states
        *self.connections.lock().unwrap() = conn_states;

        // Spawn management task
        let management_handle = tokio::spawn(management_task(
            self.connections.clone(),
            self.pool_config.clone(),
            self.reconnection_config.clone(),
            self.stream_factory.clone(),
            self.event_tx.clone(),
            self.exchange_name,
        ));
        self.management_handle = Some(management_handle);

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
        // Abort management task
        if let Some(handle) = &self.management_handle {
            handle.abort();
        }

        // Abort all connection tasks
        let conns = self.connections.lock().unwrap();
        for conn in conns.iter() {
            conn.handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::atomic::AtomicU32;
    use std::sync::Mutex as StdMutex;

    use crate::core::domain::OrderBook;

    /// Mock data stream for testing.
    struct MockDataStream {
        /// Events to return from next_event
        events: Arc<StdMutex<VecDeque<Option<MarketEvent>>>>,
        /// Whether to cycle events indefinitely
        cycle_events: bool,
        /// Original events for cycling
        original_events: Arc<StdMutex<Vec<Option<MarketEvent>>>>,
        /// Number of times connect was called
        connect_count: Arc<AtomicU32>,
        /// Number of times subscribe was called
        subscribe_count: Arc<AtomicU32>,
        /// Tokens that were subscribed
        subscribed_tokens: Arc<StdMutex<Vec<Vec<TokenId>>>>,
    }

    impl MockDataStream {
        fn new() -> Self {
            Self {
                events: Arc::new(StdMutex::new(VecDeque::new())),
                cycle_events: false,
                original_events: Arc::new(StdMutex::new(Vec::new())),
                connect_count: Arc::new(AtomicU32::new(0)),
                subscribe_count: Arc::new(AtomicU32::new(0)),
                subscribed_tokens: Arc::new(StdMutex::new(Vec::new())),
            }
        }

        fn with_events(self, events: Vec<Option<MarketEvent>>) -> Self {
            *self.events.lock().unwrap() = events.clone().into();
            *self.original_events.lock().unwrap() = events;
            self
        }

        fn with_cycle_events(mut self, events: Vec<Option<MarketEvent>>) -> Self {
            *self.events.lock().unwrap() = events.clone().into();
            *self.original_events.lock().unwrap() = events;
            self.cycle_events = true;
            self
        }

        #[allow(dead_code)]
        fn subscribe_count(&self) -> u32 {
            self.subscribe_count.load(Ordering::SeqCst)
        }

        #[allow(dead_code)]
        fn subscribed_tokens(&self) -> Vec<Vec<TokenId>> {
            self.subscribed_tokens.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl MarketDataStream for MockDataStream {
        async fn connect(&mut self) -> Result<()> {
            self.connect_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn subscribe(&mut self, token_ids: &[TokenId]) -> Result<()> {
            self.subscribe_count.fetch_add(1, Ordering::SeqCst);
            self.subscribed_tokens
                .lock()
                .unwrap()
                .push(token_ids.to_vec());
            Ok(())
        }

        async fn next_event(&mut self) -> Option<MarketEvent> {
            let event = {
                let mut events = self.events.lock().unwrap();
                let event = events.pop_front().flatten();

                // If cycling is enabled and we ran out of events, reload from original
                if self.cycle_events && events.is_empty() {
                    let original = self.original_events.lock().unwrap();
                    *events = original.clone().into();
                }

                event
            }; // Lock dropped here

            // Add small delay to avoid tight loop
            if event.is_some() {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }

            event
        }

        fn exchange_name(&self) -> &'static str {
            "mock"
        }
    }

    fn test_pool_config(
        max_connections: usize,
        subscriptions_per_connection: usize,
    ) -> ConnectionPoolConfig {
        ConnectionPoolConfig {
            connection_ttl_secs: 120,
            preemptive_reconnect_secs: 30,
            health_check_interval_secs: 30,
            max_silent_secs: 60,
            max_connections,
            subscriptions_per_connection,
        }
    }

    fn test_reconnection_config() -> ReconnectionConfig {
        ReconnectionConfig {
            initial_delay_ms: 10,
            max_delay_ms: 100,
            backoff_multiplier: 2.0,
            max_consecutive_failures: 3,
            circuit_breaker_cooldown_ms: 50,
        }
    }

    #[tokio::test]
    async fn test_pool_single_connection() {
        let mock_events = Arc::new(StdMutex::new(VecDeque::new()));
        let connect_count = Arc::new(AtomicU32::new(0));
        let subscribe_count = Arc::new(AtomicU32::new(0));

        let events_clone = Arc::clone(&mock_events);
        let connect_clone = Arc::clone(&connect_count);
        let subscribe_clone = Arc::clone(&subscribe_count);

        let factory: StreamFactory = Arc::new(move || {
            let mut mock = MockDataStream::new();
            mock.events = Arc::clone(&events_clone);
            mock.connect_count = Arc::clone(&connect_clone);
            mock.subscribe_count = Arc::clone(&subscribe_clone);
            Box::new(mock)
        });

        let pool_config = test_pool_config(10, 500);
        let mut pool = ConnectionPool::new(
            pool_config,
            test_reconnection_config(),
            factory,
            "test",
        );

        // Subscribe to 10 tokens (should use 1 connection)
        let tokens: Vec<TokenId> = (0..10)
            .map(|i| TokenId::from(format!("token{i}")))
            .collect();
        pool.subscribe(&tokens).await.unwrap();

        // Give tasks time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Should have spawned 1 connection
        let conns = pool.connections.lock().unwrap();
        assert_eq!(conns.len(), 1);
        assert_eq!(pool.subscription_map.len(), 1);
        assert_eq!(pool.subscription_map[0].len(), 10);
    }

    #[tokio::test]
    async fn test_pool_multiple_connections() {
        let mock_events = Arc::new(StdMutex::new(VecDeque::new()));
        let connect_count = Arc::new(AtomicU32::new(0));
        let subscribe_count = Arc::new(AtomicU32::new(0));

        let events_clone = Arc::clone(&mock_events);
        let connect_clone = Arc::clone(&connect_count);
        let subscribe_clone = Arc::clone(&subscribe_count);

        let factory: StreamFactory = Arc::new(move || {
            let mut mock = MockDataStream::new();
            mock.events = Arc::clone(&events_clone);
            mock.connect_count = Arc::clone(&connect_clone);
            mock.subscribe_count = Arc::clone(&subscribe_clone);
            Box::new(mock)
        });

        let pool_config = test_pool_config(10, 500);
        let mut pool = ConnectionPool::new(
            pool_config,
            test_reconnection_config(),
            factory,
            "test",
        );

        // Subscribe to 1000 tokens (should use 2 connections: 500 + 500)
        let tokens: Vec<TokenId> = (0..1000)
            .map(|i| TokenId::from(format!("token{i}")))
            .collect();
        pool.subscribe(&tokens).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Should have spawned 2 connections
        let conns = pool.connections.lock().unwrap();
        assert_eq!(conns.len(), 2);
        assert_eq!(pool.subscription_map.len(), 2);
        assert_eq!(pool.subscription_map[0].len(), 500);
        assert_eq!(pool.subscription_map[1].len(), 500);
    }

    #[tokio::test]
    async fn test_pool_respects_max_connections() {
        let mock_events = Arc::new(StdMutex::new(VecDeque::new()));
        let connect_count = Arc::new(AtomicU32::new(0));
        let subscribe_count = Arc::new(AtomicU32::new(0));

        let events_clone = Arc::clone(&mock_events);
        let connect_clone = Arc::clone(&connect_count);
        let subscribe_clone = Arc::clone(&subscribe_count);

        let factory: StreamFactory = Arc::new(move || {
            let mut mock = MockDataStream::new();
            mock.events = Arc::clone(&events_clone);
            mock.connect_count = Arc::clone(&connect_clone);
            mock.subscribe_count = Arc::clone(&subscribe_clone);
            Box::new(mock)
        });

        let pool_config = test_pool_config(3, 500); // Max 3 connections
        let mut pool = ConnectionPool::new(
            pool_config,
            test_reconnection_config(),
            factory,
            "test",
        );

        // Subscribe to 5000 tokens (would need 10 connections, but capped at 3)
        let tokens: Vec<TokenId> = (0..5000)
            .map(|i| TokenId::from(format!("token{i}")))
            .collect();
        pool.subscribe(&tokens).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Should have spawned exactly 3 connections (capped)
        let conns = pool.connections.lock().unwrap();
        assert_eq!(conns.len(), 3);
        assert_eq!(pool.subscription_map.len(), 3);

        // All tokens should be distributed across the 3 connections
        let total_subscribed: usize = pool.subscription_map.iter().map(Vec::len).sum();
        assert_eq!(total_subscribed, 5000);
    }

    #[tokio::test]
    async fn test_pool_merges_events() {
        let events = vec![
            Some(MarketEvent::OrderBookSnapshot {
                token_id: TokenId::from("token1".to_string()),
                book: OrderBook::new(TokenId::from("token1".to_string())),
            }),
            Some(MarketEvent::OrderBookSnapshot {
                token_id: TokenId::from("token2".to_string()),
                book: OrderBook::new(TokenId::from("token2".to_string())),
            }),
        ];

        let mock_events = Arc::new(StdMutex::new(VecDeque::from(events)));
        let connect_count = Arc::new(AtomicU32::new(0));
        let subscribe_count = Arc::new(AtomicU32::new(0));

        let events_clone = Arc::clone(&mock_events);
        let connect_clone = Arc::clone(&connect_count);
        let subscribe_clone = Arc::clone(&subscribe_count);

        let factory: StreamFactory = Arc::new(move || {
            let mut mock = MockDataStream::new();
            mock.events = Arc::clone(&events_clone);
            mock.connect_count = Arc::clone(&connect_clone);
            mock.subscribe_count = Arc::clone(&subscribe_clone);
            Box::new(mock)
        });

        let pool_config = test_pool_config(10, 500);
        let mut pool = ConnectionPool::new(
            pool_config,
            test_reconnection_config(),
            factory,
            "test",
        );

        let tokens = vec![TokenId::from("token1".to_string())];
        pool.subscribe(&tokens).await.unwrap();

        // Give task time to start and send events
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Should be able to receive events from the pool
        let event1 = pool.next_event().await;
        assert!(matches!(
            event1,
            Some(MarketEvent::OrderBookSnapshot { .. })
        ));

        let event2 = pool.next_event().await;
        assert!(matches!(
            event2,
            Some(MarketEvent::OrderBookSnapshot { .. })
        ));
    }

    #[tokio::test]
    async fn test_pool_exchange_name() {
        let factory: StreamFactory = Arc::new(|| Box::new(MockDataStream::new()));
        let pool_config = test_pool_config(10, 500);

        // Test with different exchange names to prove it's generic
        let pool_polymarket = ConnectionPool::new(
            pool_config.clone(),
            test_reconnection_config(),
            factory.clone(),
            "polymarket",
        );
        assert_eq!(pool_polymarket.exchange_name(), "polymarket");

        let pool_custom = ConnectionPool::new(
            pool_config,
            test_reconnection_config(),
            factory,
            "custom_exchange",
        );
        assert_eq!(pool_custom.exchange_name(), "custom_exchange");
    }

    #[tokio::test]
    async fn test_pool_connect_is_noop() {
        let factory: StreamFactory = Arc::new(|| Box::new(MockDataStream::new()));
        let pool_config = test_pool_config(10, 500);
        let mut pool = ConnectionPool::new(
            pool_config,
            test_reconnection_config(),
            factory,
            "test",
        );

        // connect() should succeed without doing anything
        let result = pool.connect().await;
        assert!(result.is_ok());

        // No tasks should be spawned
        let conns = pool.connections.lock().unwrap();
        assert_eq!(conns.len(), 0);
    }

    #[tokio::test]
    async fn test_pool_empty_subscribe() {
        let factory: StreamFactory = Arc::new(|| Box::new(MockDataStream::new()));
        let pool_config = test_pool_config(10, 500);
        let mut pool = ConnectionPool::new(
            pool_config,
            test_reconnection_config(),
            factory,
            "test",
        );

        // Subscribe with empty token list
        let result = pool.subscribe(&[]).await;
        assert!(result.is_ok());

        // No tasks should be spawned
        let conns = pool.connections.lock().unwrap();
        assert_eq!(conns.len(), 0);
    }

    #[tokio::test]
    async fn test_pool_subscribe_distributes_evenly() {
        let factory: StreamFactory = Arc::new(|| Box::new(MockDataStream::new()));
        let pool_config = test_pool_config(10, 500);
        let mut pool = ConnectionPool::new(
            pool_config,
            test_reconnection_config(),
            factory,
            "test",
        );

        // Subscribe to 1250 tokens (should use 3 connections: 500 + 500 + 250)
        let tokens: Vec<TokenId> = (0..1250)
            .map(|i| TokenId::from(format!("token{i}")))
            .collect();
        pool.subscribe(&tokens).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Verify distribution
        assert_eq!(pool.subscription_map.len(), 3);
        assert_eq!(pool.subscription_map[0].len(), 500);
        assert_eq!(pool.subscription_map[1].len(), 500);
        assert_eq!(pool.subscription_map[2].len(), 250);
    }

    // New tests for TTL rotation and health monitoring

    #[tokio::test]
    async fn test_pool_ttl_rotation() {
        let connect_count = Arc::new(AtomicU32::new(0));
        let connect_clone = Arc::clone(&connect_count);

        let factory: StreamFactory = Arc::new(move || {
            let mut mock = MockDataStream::new();
            mock.connect_count = Arc::clone(&connect_clone);
            // Mock that sends events continuously
            let events = vec![Some(MarketEvent::OrderBookSnapshot {
                token_id: TokenId::from("token1".to_string()),
                book: OrderBook::new(TokenId::from("token1".to_string())),
            })];
            Box::new(mock.with_cycle_events(events))
        });

        let mut pool_config = test_pool_config(10, 500);
        pool_config.connection_ttl_secs = 1;
        pool_config.preemptive_reconnect_secs = 0;
        pool_config.health_check_interval_secs = 1;

        let mut pool = ConnectionPool::new(
            pool_config,
            test_reconnection_config(),
            factory,
            "test",
        );

        let tokens = vec![TokenId::from("token1".to_string())];
        pool.subscribe(&tokens).await.unwrap();

        // Wait for TTL rotation to happen
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        // Connection should have been replaced at least once
        let final_count = connect_count.load(Ordering::SeqCst);
        assert!(
            final_count > 1,
            "Expected connection to be rotated, but connect was only called {} times",
            final_count
        );
    }

    #[tokio::test]
    async fn test_pool_silent_death_detection() {
        let connect_count = Arc::new(AtomicU32::new(0));
        let connect_clone = Arc::clone(&connect_count);

        let factory: StreamFactory = Arc::new(move || {
            let mut mock = MockDataStream::new();
            mock.connect_count = Arc::clone(&connect_clone);
            // Mock that sends 1 event then stops
            let events = vec![Some(MarketEvent::OrderBookSnapshot {
                token_id: TokenId::from("token1".to_string()),
                book: OrderBook::new(TokenId::from("token1".to_string())),
            })];
            Box::new(mock.with_events(events))
        });

        let mut pool_config = test_pool_config(10, 500);
        pool_config.max_silent_secs = 1;
        pool_config.health_check_interval_secs = 1;
        pool_config.connection_ttl_secs = 120; // High TTL so only silent death triggers

        let mut pool = ConnectionPool::new(
            pool_config,
            test_reconnection_config(),
            factory,
            "test",
        );

        let tokens = vec![TokenId::from("token1".to_string())];
        pool.subscribe(&tokens).await.unwrap();

        // Wait for first event and then for silent death detection
        tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;

        // Connection should have been restarted due to silence
        let final_count = connect_count.load(Ordering::SeqCst);
        assert!(
            final_count > 1,
            "Expected connection to be restarted due to silence, but connect was only called {} times",
            final_count
        );
    }

    #[tokio::test]
    async fn test_pool_preemptive_reconnect() {
        let connect_count = Arc::new(AtomicU32::new(0));
        let connect_clone = Arc::clone(&connect_count);

        let factory: StreamFactory = Arc::new(move || {
            let mut mock = MockDataStream::new();
            mock.connect_count = Arc::clone(&connect_clone);
            // Continuously send events
            let events = vec![Some(MarketEvent::OrderBookSnapshot {
                token_id: TokenId::from("token1".to_string()),
                book: OrderBook::new(TokenId::from("token1".to_string())),
            })];
            Box::new(mock.with_cycle_events(events))
        });

        let mut pool_config = test_pool_config(10, 500);
        pool_config.connection_ttl_secs = 3;
        pool_config.preemptive_reconnect_secs = 2;
        pool_config.health_check_interval_secs = 1;

        let mut pool = ConnectionPool::new(
            pool_config,
            test_reconnection_config(),
            factory,
            "test",
        );

        let tokens = vec![TokenId::from("token1".to_string())];
        pool.subscribe(&tokens).await.unwrap();

        // Wait ~1-2 seconds (ttl - preemptive = 3 - 2 = 1)
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Should have reconnected preemptively before TTL
        let count_at_2s = connect_count.load(Ordering::SeqCst);
        assert!(
            count_at_2s > 1,
            "Expected preemptive reconnect at ~1s, but connect was only called {} times after 2s",
            count_at_2s
        );
    }

    #[tokio::test]
    async fn test_pool_crashed_task_restart() {
        let connect_count = Arc::new(AtomicU32::new(0));
        let connect_clone = Arc::clone(&connect_count);

        let factory: StreamFactory = Arc::new(move || {
            let mut mock = MockDataStream::new();
            mock.connect_count = Arc::clone(&connect_clone);
            // Mock that immediately returns None (stream ends)
            Box::new(mock.with_events(vec![]))
        });

        let mut pool_config = test_pool_config(10, 500);
        pool_config.health_check_interval_secs = 1;

        let mut pool = ConnectionPool::new(
            pool_config,
            test_reconnection_config(),
            factory,
            "test",
        );

        let tokens = vec![TokenId::from("token1".to_string())];
        pool.subscribe(&tokens).await.unwrap();

        // Wait for management task to detect and restart crashed connection
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        // Connection should have been restarted multiple times
        let final_count = connect_count.load(Ordering::SeqCst);
        assert!(
            final_count > 1,
            "Expected crashed connection to be restarted, but connect was only called {} times",
            final_count
        );
    }

    #[tokio::test]
    async fn test_pool_healthy_connection_not_replaced() {
        let connect_count = Arc::new(AtomicU32::new(0));
        let connect_clone = Arc::clone(&connect_count);

        let factory: StreamFactory = Arc::new(move || {
            let mut mock = MockDataStream::new();
            mock.connect_count = Arc::clone(&connect_clone);
            // Mock that sends events continuously
            let events = vec![Some(MarketEvent::OrderBookSnapshot {
                token_id: TokenId::from("token1".to_string()),
                book: OrderBook::new(TokenId::from("token1".to_string())),
            })];
            Box::new(mock.with_cycle_events(events))
        });

        let mut pool_config = test_pool_config(10, 500);
        pool_config.connection_ttl_secs = 120; // Long TTL
        pool_config.max_silent_secs = 60; // Long silence threshold
        pool_config.health_check_interval_secs = 1;

        let mut pool = ConnectionPool::new(
            pool_config,
            test_reconnection_config(),
            factory,
            "test",
        );

        let tokens = vec![TokenId::from("token1".to_string())];
        pool.subscribe(&tokens).await.unwrap();

        // Wait and verify connection is not needlessly replaced
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        // Should only have connected once (no rotation)
        let final_count = connect_count.load(Ordering::SeqCst);
        assert_eq!(
            final_count, 1,
            "Expected healthy connection to stay alive, but connect was called {} times",
            final_count
        );
    }
}
