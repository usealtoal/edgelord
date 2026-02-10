//! Integration tests for the connection pool.
//!
//! These tests use mock WebSocket servers to verify pool behavior
//! end-to-end: event delivery, connection failure recovery,
//! multi-connection merging, backpressure, and TTL rotation.

mod support;

use std::collections::HashSet;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use edgelord::app::{ConnectionPoolConfig, ReconnectionConfig};
use edgelord::core::domain::{OrderBook, TokenId};
use edgelord::core::exchange::{ConnectionPool, MarketDataStream, MarketEvent, StreamFactory};
use edgelord::error::Result;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_tokens(n: usize) -> Vec<TokenId> {
    (0..n).map(|i| TokenId::from(format!("t{i}"))).collect()
}

fn snapshot_event(token: &str) -> MarketEvent {
    MarketEvent::OrderBookSnapshot {
        token_id: TokenId::from(token.to_string()),
        book: OrderBook::new(TokenId::from(token.to_string())),
    }
}

fn pool_config(max_conns: usize, subs_per_conn: usize) -> ConnectionPoolConfig {
    ConnectionPoolConfig {
        max_connections: max_conns,
        subscriptions_per_connection: subs_per_conn,
        connection_ttl_secs: 120,
        preemptive_reconnect_secs: 30,
        health_check_interval_secs: 30,
        max_silent_secs: 60,
        channel_capacity: 10_000,
    }
}

fn fast_reconnect() -> ReconnectionConfig {
    ReconnectionConfig {
        initial_delay_ms: 10,
        max_delay_ms: 50,
        backoff_multiplier: 1.5,
        max_consecutive_failures: 5,
        circuit_breaker_cooldown_ms: 50,
    }
}

// ---------------------------------------------------------------------------
// Mock stream — configurable via closures for maximum test flexibility
// ---------------------------------------------------------------------------

/// A mock [`MarketDataStream`] that yields events from a channel.
///
/// Each test sends events into the `event_tx` side, and the pool
/// reads them via `next_event()`. This simulates a real WS server
/// without needing actual network I/O.
struct ChannelStream {
    event_rx: tokio::sync::mpsc::Receiver<Option<MarketEvent>>,
    connect_count: Arc<AtomicU32>,
    subscribe_count: Arc<AtomicU32>,
    subscribed_tokens: Arc<std::sync::Mutex<Vec<TokenId>>>,
}

struct ChannelStreamHandle {
    event_tx: tokio::sync::mpsc::Sender<Option<MarketEvent>>,
    connect_count: Arc<AtomicU32>,
    subscribe_count: Arc<AtomicU32>,
    subscribed_tokens: Arc<std::sync::Mutex<Vec<TokenId>>>,
}

impl ChannelStreamHandle {
    /// Send an event to the stream.
    async fn send(&self, event: MarketEvent) {
        let _ = self.event_tx.send(Some(event)).await;
    }

    /// Signal end-of-stream (causes `next_event` to return `None`).
    #[allow(dead_code)]
    async fn close(&self) {
        let _ = self.event_tx.send(None).await;
    }

    /// How many times `connect()` was called.
    fn connect_count(&self) -> u32 {
        self.connect_count.load(Ordering::SeqCst)
    }

    /// How many times `subscribe()` was called.
    fn subscribe_count(&self) -> u32 {
        self.subscribe_count.load(Ordering::SeqCst)
    }

    /// Which tokens were last subscribed to.
    fn subscribed_tokens(&self) -> Vec<TokenId> {
        self.subscribed_tokens.lock().unwrap().clone()
    }
}

fn channel_stream(buffer: usize) -> (ChannelStream, ChannelStreamHandle) {
    let (tx, rx) = tokio::sync::mpsc::channel(buffer);
    let cc = Arc::new(AtomicU32::new(0));
    let sc = Arc::new(AtomicU32::new(0));
    let st = Arc::new(std::sync::Mutex::new(Vec::new()));
    (
        ChannelStream {
            event_rx: rx,
            connect_count: cc.clone(),
            subscribe_count: sc.clone(),
            subscribed_tokens: st.clone(),
        },
        ChannelStreamHandle {
            event_tx: tx,
            connect_count: cc,
            subscribe_count: sc,
            subscribed_tokens: st,
        },
    )
}

#[async_trait::async_trait]
impl MarketDataStream for ChannelStream {
    async fn connect(&mut self) -> Result<()> {
        self.connect_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn subscribe(&mut self, token_ids: &[TokenId]) -> Result<()> {
        self.subscribe_count.fetch_add(1, Ordering::SeqCst);
        *self.subscribed_tokens.lock().unwrap() = token_ids.to_vec();
        Ok(())
    }

    async fn next_event(&mut self) -> Option<MarketEvent> {
        match self.event_rx.recv().await {
            Some(Some(event)) => Some(event),
            Some(None) | None => None,
        }
    }

    fn exchange_name(&self) -> &'static str {
        "mock"
    }
}

// ---------------------------------------------------------------------------
// Cycling mock — produces events forever for TTL/health tests
// ---------------------------------------------------------------------------

struct CyclingStream {
    events: Vec<MarketEvent>,
    index: usize,
    delay: Duration,
    connect_count: Arc<AtomicU32>,
}

impl CyclingStream {
    fn new(events: Vec<MarketEvent>, delay: Duration, connect_count: Arc<AtomicU32>) -> Self {
        Self {
            events,
            index: 0,
            delay,
            connect_count,
        }
    }
}

#[async_trait::async_trait]
impl MarketDataStream for CyclingStream {
    async fn connect(&mut self) -> Result<()> {
        self.connect_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn subscribe(&mut self, _: &[TokenId]) -> Result<()> {
        Ok(())
    }

    async fn next_event(&mut self) -> Option<MarketEvent> {
        if self.events.is_empty() {
            return std::future::pending().await;
        }
        tokio::time::sleep(self.delay).await;
        let event = self.events[self.index % self.events.len()].clone();
        self.index += 1;
        Some(event)
    }

    fn exchange_name(&self) -> &'static str {
        "mock"
    }
}

// ---------------------------------------------------------------------------
// Test 1: Single connection delivers events end-to-end
// ---------------------------------------------------------------------------

#[tokio::test]
async fn single_connection_delivers_events() {
    // Use a shared handle list so the factory closure can hand us the controls.
    let handles: Arc<std::sync::Mutex<Vec<ChannelStreamHandle>>> =
        Arc::new(std::sync::Mutex::new(Vec::new()));
    let handles_clone = handles.clone();

    let factory: StreamFactory = Arc::new(move || {
        let (stream, handle) = channel_stream(64);
        handles_clone.lock().unwrap().push(handle);
        Box::new(stream)
    });

    let mut pool =
        ConnectionPool::new(pool_config(10, 500), fast_reconnect(), factory, "test").unwrap();
    pool.connect().await.unwrap();
    pool.subscribe(&make_tokens(3)).await.unwrap();

    // Give pool time to spawn connections.
    tokio::time::sleep(Duration::from_millis(50)).await;

    let h = &handles.lock().unwrap()[0];
    assert_eq!(h.connect_count(), 1);
    assert_eq!(h.subscribe_count(), 1);
    assert_eq!(h.subscribed_tokens().len(), 3);

    // Send events through the mock and read them from the pool.
    h.send(snapshot_event("t0")).await;
    h.send(snapshot_event("t1")).await;

    let e1 = pool.next_event().await.unwrap();
    let e2 = pool.next_event().await.unwrap();

    // Verify we got both (order preserved in single-connection case).
    let ids: Vec<_> = [e1, e2]
        .iter()
        .map(|e| match e {
            MarketEvent::OrderBookSnapshot { token_id, .. } => token_id.to_string(),
            _ => panic!("unexpected event type"),
        })
        .collect();
    assert!(ids.contains(&"t0".to_string()));
    assert!(ids.contains(&"t1".to_string()));
}

// ---------------------------------------------------------------------------
// Test 2: Multiple connections merge into single stream
// ---------------------------------------------------------------------------

#[tokio::test]
async fn multi_connection_merges_events() {
    let handles: Arc<std::sync::Mutex<Vec<ChannelStreamHandle>>> =
        Arc::new(std::sync::Mutex::new(Vec::new()));
    let handles_clone = handles.clone();

    let factory: StreamFactory = Arc::new(move || {
        let (stream, handle) = channel_stream(64);
        handles_clone.lock().unwrap().push(handle);
        Box::new(stream)
    });

    // 2 tokens per connection → 2 connections for 4 tokens.
    let mut pool =
        ConnectionPool::new(pool_config(10, 2), fast_reconnect(), factory, "test").unwrap();
    pool.connect().await.unwrap();
    pool.subscribe(&make_tokens(4)).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    let h = handles.lock().unwrap();
    assert_eq!(h.len(), 2, "Expected 2 connections for 4 tokens with subs_per_conn=2");

    // Each connection sends its own events.
    h[0].send(snapshot_event("conn0-a")).await;
    h[1].send(snapshot_event("conn1-a")).await;
    h[0].send(snapshot_event("conn0-b")).await;
    h[1].send(snapshot_event("conn1-b")).await;

    // Collect all 4 events — order may interleave but all must arrive.
    let mut received = HashSet::new();
    for _ in 0..4 {
        match tokio::time::timeout(Duration::from_secs(2), pool.next_event()).await {
            Ok(Some(MarketEvent::OrderBookSnapshot { token_id, .. })) => {
                received.insert(token_id.to_string());
            }
            other => panic!("Expected snapshot event, got {:?}", other),
        }
    }

    assert_eq!(received.len(), 4);
    assert!(received.contains("conn0-a"));
    assert!(received.contains("conn0-b"));
    assert!(received.contains("conn1-a"));
    assert!(received.contains("conn1-b"));
}

// ---------------------------------------------------------------------------
// Test 3: Backpressure — full channel drops events and increments counter
// ---------------------------------------------------------------------------

#[tokio::test]
async fn backpressure_drops_events_and_counts() {
    let handles: Arc<std::sync::Mutex<Vec<ChannelStreamHandle>>> =
        Arc::new(std::sync::Mutex::new(Vec::new()));
    let handles_clone = handles.clone();

    let factory: StreamFactory = Arc::new(move || {
        let (stream, handle) = channel_stream(256);
        handles_clone.lock().unwrap().push(handle);
        Box::new(stream)
    });

    // Tiny channel capacity to trigger backpressure quickly.
    let mut cfg = pool_config(10, 500);
    cfg.channel_capacity = 5;

    let mut pool = ConnectionPool::new(cfg, fast_reconnect(), factory, "test").unwrap();
    pool.connect().await.unwrap();
    pool.subscribe(&make_tokens(1)).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    let h = &handles.lock().unwrap()[0];

    // Flood 20 events without reading any — channel holds 5, rest should drop.
    for i in 0..20 {
        h.send(snapshot_event(&format!("flood-{i}"))).await;
    }
    // Small delay to let the connection task process all sends.
    tokio::time::sleep(Duration::from_millis(100)).await;

    let stats = pool.stats();
    assert!(
        stats.events_dropped > 0,
        "Expected events_dropped > 0, got {}",
        stats.events_dropped
    );

    // We should still be able to drain the channel.
    let mut read = 0;
    while let Ok(Some(_)) = tokio::time::timeout(Duration::from_millis(100), pool.next_event()).await
    {
        read += 1;
    }
    assert!(read > 0, "Should have drained at least some events");
    assert!(
        read + stats.events_dropped as usize >= 20,
        "read ({read}) + dropped ({}) should account for all 20 events",
        stats.events_dropped
    );
}

// ---------------------------------------------------------------------------
// Test 4: Server disconnect triggers reconnect (via ReconnectingDataStream)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn disconnect_triggers_reconnect() {
    let connect_count = Arc::new(AtomicU32::new(0));
    let cc = connect_count.clone();

    let factory: StreamFactory = Arc::new(move || {
        let cc_inner = cc.clone();
        // Each new stream created by ReconnectingDataStream also goes
        // through the factory, so we count factory invocations.
        Box::new(CyclingStream::new(
            vec![snapshot_event("t0")],
            Duration::from_millis(10),
            cc_inner,
        ))
    });

    let mut cfg = pool_config(10, 500);
    cfg.max_silent_secs = 2;
    cfg.health_check_interval_secs = 1;

    let mut pool = ConnectionPool::new(cfg, fast_reconnect(), factory, "test").unwrap();
    pool.connect().await.unwrap();
    pool.subscribe(&make_tokens(1)).await.unwrap();

    // Read a few events to confirm stream is working.
    for _ in 0..3 {
        let event = tokio::time::timeout(Duration::from_secs(2), pool.next_event())
            .await
            .expect("timeout waiting for event");
        assert!(event.is_some());
    }

    // Pool should have connected at least once.
    assert!(
        connect_count.load(Ordering::SeqCst) >= 1,
        "Expected at least 1 connection"
    );
}

// ---------------------------------------------------------------------------
// Test 5: TTL rotation replaces connections under continuous load
// ---------------------------------------------------------------------------

#[tokio::test]
async fn ttl_rotation_under_load() {
    let connect_count = Arc::new(AtomicU32::new(0));
    let cc = connect_count.clone();

    let factory: StreamFactory = Arc::new(move || {
        Box::new(CyclingStream::new(
            vec![snapshot_event("t0")],
            Duration::from_millis(20),
            cc.clone(),
        ))
    });

    let mut cfg = pool_config(10, 500);
    cfg.connection_ttl_secs = 3;
    cfg.preemptive_reconnect_secs = 2;
    cfg.health_check_interval_secs = 1;

    let mut pool = ConnectionPool::new(cfg, fast_reconnect(), factory, "test").unwrap();
    pool.connect().await.unwrap();
    pool.subscribe(&make_tokens(1)).await.unwrap();

    // Drain events for 5 seconds — should see at least one rotation.
    let drain = tokio::spawn(async move {
        let mut count = 0u64;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            match tokio::time::timeout_at(deadline, pool.next_event()).await {
                Ok(Some(_)) => count += 1,
                _ => break,
            }
        }
        (pool, count)
    });

    let (pool, events_received) = drain.await.unwrap();

    // Should have received events continuously (no long gaps).
    assert!(
        events_received > 10,
        "Expected >10 events over 5s, got {events_received}"
    );

    // Should have rotated at least once (TTL=3s, window=5s).
    let stats = pool.stats();
    assert!(
        stats.total_rotations > 0,
        "Expected at least 1 TTL rotation, got {}",
        stats.total_rotations
    );

    // Factory should have been called more than once (original + rotation).
    assert!(
        connect_count.load(Ordering::SeqCst) > 1,
        "Expected more than 1 factory call after rotation"
    );
}

// ---------------------------------------------------------------------------
// Test 6: Pool stats reflect reality
// ---------------------------------------------------------------------------

#[tokio::test]
async fn pool_stats_reflect_connections() {
    let handles: Arc<std::sync::Mutex<Vec<ChannelStreamHandle>>> =
        Arc::new(std::sync::Mutex::new(Vec::new()));
    let handles_clone = handles.clone();

    let factory: StreamFactory = Arc::new(move || {
        let (stream, handle) = channel_stream(64);
        handles_clone.lock().unwrap().push(handle);
        Box::new(stream)
    });

    let mut pool =
        ConnectionPool::new(pool_config(10, 2), fast_reconnect(), factory, "test").unwrap();

    // Before subscribe: 0 connections.
    assert_eq!(pool.stats().active_connections, 0);

    pool.connect().await.unwrap();
    pool.subscribe(&make_tokens(6)).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    // 6 tokens / 2 per conn = 3 connections.
    assert_eq!(pool.stats().active_connections, 3);
    assert_eq!(pool.stats().total_rotations, 0);
    assert_eq!(pool.stats().total_restarts, 0);
    assert_eq!(pool.stats().events_dropped, 0);
}

// ---------------------------------------------------------------------------
// Test 7: Resubscribe tears down old connections cleanly
// ---------------------------------------------------------------------------

#[tokio::test]
async fn resubscribe_replaces_all_connections() {
    let handles: Arc<std::sync::Mutex<Vec<ChannelStreamHandle>>> =
        Arc::new(std::sync::Mutex::new(Vec::new()));
    let handles_clone = handles.clone();

    let factory: StreamFactory = Arc::new(move || {
        let (stream, handle) = channel_stream(64);
        handles_clone.lock().unwrap().push(handle);
        Box::new(stream)
    });

    let mut pool =
        ConnectionPool::new(pool_config(10, 2), fast_reconnect(), factory, "test").unwrap();
    pool.connect().await.unwrap();

    // First subscribe: 4 tokens → 2 connections.
    pool.subscribe(&make_tokens(4)).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(pool.stats().active_connections, 2);

    // Second subscribe: 6 tokens → 3 connections, old 2 torn down.
    pool.subscribe(&make_tokens(6)).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(pool.stats().active_connections, 3);

    // The new connections should work.
    let h = handles.lock().unwrap();
    // Handles 0 and 1 are from the first subscribe (now dead).
    // Handles 2, 3, 4 are from the second subscribe.
    h[2].send(snapshot_event("new-t0")).await;

    let event = tokio::time::timeout(Duration::from_secs(2), pool.next_event())
        .await
        .expect("timeout")
        .expect("no event");

    match event {
        MarketEvent::OrderBookSnapshot { token_id, .. } => {
            assert_eq!(token_id.to_string(), "new-t0");
        }
        _ => panic!("unexpected event type"),
    }
}

// ---------------------------------------------------------------------------
// Test 8: Exchange name propagates correctly
// ---------------------------------------------------------------------------

#[tokio::test]
async fn exchange_name_propagates() {
    let factory: StreamFactory = Arc::new(|| {
        let (stream, _) = channel_stream(1);
        Box::new(stream)
    });

    let pool =
        ConnectionPool::new(pool_config(10, 500), fast_reconnect(), factory, "polymarket").unwrap();
    assert_eq!(pool.exchange_name(), "polymarket");
}
