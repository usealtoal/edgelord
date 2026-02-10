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
//! This design avoids borrow checker issues by eliminating the need to hold
//! mutable references to connections while iterating over them.

use std::sync::Arc;

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
    /// Handles to spawned connection tasks (for cleanup).
    task_handles: Vec<tokio::task::JoinHandle<()>>,
    /// Track which tokens are assigned to which connection.
    subscription_map: Vec<Vec<TokenId>>,
    /// Exchange name for identification.
    exchange_name: &'static str,
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
            task_handles: Vec::new(),
            subscription_map: Vec::new(),
            exchange_name,
        }
    }

    /// Spawn a connection task for the given tokens.
    ///
    /// Each connection task:
    /// 1. Wraps the stream in a `ReconnectingDataStream`
    /// 2. Connects and subscribes to its assigned tokens
    /// 3. Continuously reads events and forwards them to the shared channel
    /// 4. Terminates when the stream ends or channel is closed
    fn spawn_connection_task(&self, tokens: Vec<TokenId>) -> tokio::task::JoinHandle<()> {
        let stream = (self.stream_factory)();
        let mut reconnecting_stream =
            ReconnectingDataStream::new(stream, self.reconnection_config.clone());
        let event_tx = self.event_tx.clone();
        let token_count = tokens.len();

        tokio::spawn(async move {
            debug!(tokens = token_count, "Connection task starting");

            // Connect
            if let Err(e) = reconnecting_stream.connect().await {
                error!(error = %e, "Connection task failed to connect");
                return;
            }

            // Subscribe
            if let Err(e) = reconnecting_stream.subscribe(&tokens).await {
                error!(error = %e, "Connection task failed to subscribe");
                return;
            }

            debug!(tokens = token_count, "Connection task subscribed");

            // Event loop
            loop {
                match reconnecting_stream.next_event().await {
                    Some(event) => {
                        if event_tx.send(event).is_err() {
                            debug!("Connection task: event channel closed, terminating");
                            break;
                        }
                    }
                    None => {
                        warn!("Connection task: stream ended");
                        break;
                    }
                }
            }

            debug!("Connection task terminated");
        })
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

        // Spawn connection tasks
        for (index, chunk) in token_chunks.iter().enumerate() {
            info!(
                connection = index + 1,
                tokens = chunk.len(),
                "Spawning connection task"
            );
            let handle = self.spawn_connection_task(chunk.clone());
            self.task_handles.push(handle);
            self.subscription_map.push(chunk.clone());
        }

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
        // Abort all connection tasks when the pool is dropped
        for handle in &self.task_handles {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Mutex as StdMutex;

    use crate::core::domain::OrderBook;

    /// Mock data stream for testing.
    struct MockDataStream {
        /// Events to return from next_event
        events: Arc<StdMutex<VecDeque<Option<MarketEvent>>>>,
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
                connect_count: Arc::new(AtomicU32::new(0)),
                subscribe_count: Arc::new(AtomicU32::new(0)),
                subscribed_tokens: Arc::new(StdMutex::new(Vec::new())),
            }
        }

        #[allow(dead_code)]
        fn with_events(self, events: Vec<Option<MarketEvent>>) -> Self {
            *self.events.lock().unwrap() = events.into();
            self
        }

        #[allow(dead_code)]
        fn connect_count(&self) -> u32 {
            self.connect_count.load(Ordering::SeqCst)
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
            self.events.lock().unwrap().pop_front().flatten()
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
        let mut pool = ConnectionPool::new(pool_config, test_reconnection_config(), factory, "test");

        // Subscribe to 10 tokens (should use 1 connection)
        let tokens: Vec<TokenId> = (0..10)
            .map(|i| TokenId::from(format!("token{i}")))
            .collect();
        pool.subscribe(&tokens).await.unwrap();

        // Give tasks time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Should have spawned 1 connection
        assert_eq!(pool.task_handles.len(), 1);
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
        let mut pool = ConnectionPool::new(pool_config, test_reconnection_config(), factory, "test");

        // Subscribe to 1000 tokens (should use 2 connections: 500 + 500)
        let tokens: Vec<TokenId> = (0..1000)
            .map(|i| TokenId::from(format!("token{i}")))
            .collect();
        pool.subscribe(&tokens).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Should have spawned 2 connections
        assert_eq!(pool.task_handles.len(), 2);
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
        let mut pool = ConnectionPool::new(pool_config, test_reconnection_config(), factory, "test");

        // Subscribe to 5000 tokens (would need 10 connections, but capped at 3)
        let tokens: Vec<TokenId> = (0..5000)
            .map(|i| TokenId::from(format!("token{i}")))
            .collect();
        pool.subscribe(&tokens).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Should have spawned exactly 3 connections (capped)
        assert_eq!(pool.task_handles.len(), 3);
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
        let mut pool = ConnectionPool::new(pool_config, test_reconnection_config(), factory, "test");

        let tokens = vec![TokenId::from("token1".to_string())];
        pool.subscribe(&tokens).await.unwrap();

        // Give task time to start and send events
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Should be able to receive events from the pool
        let event1 = pool.next_event().await;
        assert!(matches!(event1, Some(MarketEvent::OrderBookSnapshot { .. })));

        let event2 = pool.next_event().await;
        assert!(matches!(event2, Some(MarketEvent::OrderBookSnapshot { .. })));
    }

    #[tokio::test]
    async fn test_pool_exchange_name() {
        let factory: StreamFactory = Arc::new(|| Box::new(MockDataStream::new()));
        let pool_config = test_pool_config(10, 500);
        
        // Test with different exchange names to prove it's generic
        let pool_polymarket = ConnectionPool::new(pool_config.clone(), test_reconnection_config(), factory.clone(), "polymarket");
        assert_eq!(pool_polymarket.exchange_name(), "polymarket");
        
        let pool_custom = ConnectionPool::new(pool_config, test_reconnection_config(), factory, "custom_exchange");
        assert_eq!(pool_custom.exchange_name(), "custom_exchange");
    }

    #[tokio::test]
    async fn test_pool_connect_is_noop() {
        let factory: StreamFactory = Arc::new(|| Box::new(MockDataStream::new()));
        let pool_config = test_pool_config(10, 500);
        let mut pool = ConnectionPool::new(pool_config, test_reconnection_config(), factory, "test");

        // connect() should succeed without doing anything
        let result = pool.connect().await;
        assert!(result.is_ok());

        // No tasks should be spawned
        assert_eq!(pool.task_handles.len(), 0);
    }

    #[tokio::test]
    async fn test_pool_empty_subscribe() {
        let factory: StreamFactory = Arc::new(|| Box::new(MockDataStream::new()));
        let pool_config = test_pool_config(10, 500);
        let mut pool = ConnectionPool::new(pool_config, test_reconnection_config(), factory, "test");

        // Subscribe with empty token list
        let result = pool.subscribe(&[]).await;
        assert!(result.is_ok());

        // No tasks should be spawned
        assert_eq!(pool.task_handles.len(), 0);
    }

    #[tokio::test]
    async fn test_pool_subscribe_distributes_evenly() {
        let factory: StreamFactory = Arc::new(|| Box::new(MockDataStream::new()));
        let pool_config = test_pool_config(10, 500);
        let mut pool = ConnectionPool::new(pool_config, test_reconnection_config(), factory, "test");

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
}
