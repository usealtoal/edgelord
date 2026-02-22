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
//! Replacements use true zero-gap handoff: new connections are spawned
//! concurrently and must deliver their first event before old connections
//! are drained and aborted.

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::{debug, info};

use crate::domain::id::TokenId;
use crate::error::{ConfigError, Result};
use crate::infrastructure::config::service::{ConnectionPoolConfig, ReconnectionConfig};
use crate::port::outbound::exchange::PoolStats;
use crate::port::{outbound::exchange::MarketDataStream, outbound::exchange::MarketEvent};

mod manage;
mod replace;
mod spawn;
mod state;

use manage::management_task;
use replace::ManagementContext;
use spawn::new_connection;
use state::{lock_or_recover, ConnectionState, SharedCounters};

/// Factory function for creating new data stream instances.
///
/// Used by the connection pool to create new connections on demand.
pub type StreamFactory = Arc<dyn Fn() -> Box<dyn MarketDataStream> + Send + Sync>;

/// Duration to drain events from an old connection before aborting it.
pub(super) const DRAIN_GRACE_PERIOD: Duration = Duration::from_millis(100);

/// Polling interval during handoff (checking for first event).
pub(super) const HANDOFF_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Starting ID for management-spawned connections.
///
/// Initial connections get IDs 1, 2, 3, ... Management-spawned replacements
/// start at this value to avoid ID collisions and make logs easier to follow.
pub(super) const MANAGEMENT_CONNECTION_ID_START: u64 = 1_000_000;

/// Exchange-agnostic connection pool that manages multiple WebSocket connections.
///
/// Distributes subscriptions across connections and provides a single merged
/// event stream. Connections are automatically rotated on TTL expiry and
/// restarted if they go silent or crash.
///
/// The event channel is bounded (configurable via `channel_capacity`) to
/// prevent unbounded memory growth under backpressure.
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
    /// - `channel_capacity` must be > 0
    #[must_use = "returns Result that must be checked"]
    pub fn new(
        pool_config: ConnectionPoolConfig,
        reconnection_config: ReconnectionConfig,
        stream_factory: StreamFactory,
        exchange_name: &'static str,
    ) -> Result<Self> {
        Self::validate_config(&pool_config)?;

        let (event_tx, event_rx) = mpsc::channel(pool_config.channel_capacity);
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
        if config.channel_capacity == 0 {
            return Err(invalid("channel_capacity", "must be > 0"));
        }
        Ok(())
    }

    /// Runtime statistics for observability (e.g. Telegram `/health` command).
    pub fn stats(&self) -> PoolStats {
        let active = lock_or_recover(&self.connections).len();
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
            channel_capacity = self.pool_config.channel_capacity,
            "Creating connection pool"
        );

        let mut states = Vec::with_capacity(chunks.len());
        for (i, tokens) in chunks.into_iter().enumerate() {
            self.next_conn_id += 1;
            let id = self.next_conn_id;
            info!(
                connection = i + 1,
                connection_id = id,
                tokens = tokens.len(),
                "Spawning"
            );

            states.push(new_connection(
                id,
                tokens,
                &self.stream_factory,
                self.reconnection_config.clone(),
                self.event_tx.clone(),
                self.counters.clone(),
            ));
        }

        *lock_or_recover(&self.connections) = states;

        // Start management task.
        let ctx = ManagementContext {
            connections: self.connections.clone(),
            config: self.pool_config.clone(),
            reconnection_config: self.reconnection_config.clone(),
            factory: self.stream_factory.clone(),
            event_tx: self.event_tx.clone(),
            counters: self.counters.clone(),
        };
        self.management_handle = Some(tokio::spawn(management_task(ctx, self.exchange_name)));

        Ok(())
    }

    async fn next_event(&mut self) -> Option<MarketEvent> {
        self.event_rx.recv().await
    }

    fn exchange_name(&self) -> &'static str {
        self.exchange_name
    }

    fn pool_stats(&self) -> Option<PoolStats> {
        Some(self.stats())
    }
}

impl Drop for ConnectionPool {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests;
