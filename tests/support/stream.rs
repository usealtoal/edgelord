//! Reusable mock [`MarketDataStream`] implementations for testing.
//!
//! - [`ChannelStream`] — channel-backed stream with external control via [`ChannelStreamHandle`].
//!   Ideal for integration tests that need precise, on-demand event delivery.
//! - [`CyclingStream`] — infinite event loop with configurable delay.
//!   Ideal for timing-based tests (TTL rotation, health monitoring).
//! - [`ScriptedStream`] — pre-loaded sequence of connect/subscribe results and events.
//!   Ideal for unit-style tests that verify reconnection and error handling.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;

use edgelord::core::domain::TokenId;
use edgelord::core::exchange::{MarketDataStream, MarketEvent};
use edgelord::error::Result;

// ---------------------------------------------------------------------------
// ChannelStream — external control via mpsc
// ---------------------------------------------------------------------------

/// A mock stream controlled externally via a [`ChannelStreamHandle`].
///
/// Events are sent into the handle's `event_tx` side and read by the pool
/// or test harness via `next_event()`. No real network I/O.
pub struct ChannelStream {
    event_rx: tokio::sync::mpsc::Receiver<Option<MarketEvent>>,
    connect_count: Arc<AtomicU32>,
    subscribe_count: Arc<AtomicU32>,
    subscribed_tokens: Arc<Mutex<Vec<TokenId>>>,
}

/// Control handle for a [`ChannelStream`].
///
/// Send events, close the stream, and inspect connection/subscription counts.
pub struct ChannelStreamHandle {
    event_tx: tokio::sync::mpsc::Sender<Option<MarketEvent>>,
    connect_count: Arc<AtomicU32>,
    subscribe_count: Arc<AtomicU32>,
    subscribed_tokens: Arc<Mutex<Vec<TokenId>>>,
}

impl ChannelStreamHandle {
    /// Send an event to the stream.
    pub async fn send(&self, event: MarketEvent) {
        let _ = self.event_tx.send(Some(event)).await;
    }

    /// Signal end-of-stream (causes `next_event` to return `None`).
    pub async fn close(&self) {
        let _ = self.event_tx.send(None).await;
    }

    /// How many times `connect()` was called.
    pub fn connect_count(&self) -> u32 {
        self.connect_count.load(Ordering::SeqCst)
    }

    /// How many times `subscribe()` was called.
    pub fn subscribe_count(&self) -> u32 {
        self.subscribe_count.load(Ordering::SeqCst)
    }

    /// Which tokens were last subscribed to.
    pub fn subscribed_tokens(&self) -> Vec<TokenId> {
        self.subscribed_tokens.lock().unwrap().clone()
    }
}

/// Create a [`ChannelStream`] and its control [`ChannelStreamHandle`].
pub fn channel_stream(buffer: usize) -> (ChannelStream, ChannelStreamHandle) {
    let (tx, rx) = tokio::sync::mpsc::channel(buffer);
    let cc = Arc::new(AtomicU32::new(0));
    let sc = Arc::new(AtomicU32::new(0));
    let st = Arc::new(Mutex::new(Vec::new()));
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

#[async_trait]
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
// CyclingStream — infinite event loop
// ---------------------------------------------------------------------------

/// A mock stream that yields events from a fixed list in an infinite loop.
///
/// Each call to `next_event()` sleeps for `delay` before returning the next
/// event. Useful for timing-based tests like TTL rotation and health monitoring.
pub struct CyclingStream {
    events: Vec<MarketEvent>,
    index: usize,
    delay: Duration,
    connect_count: Arc<AtomicU32>,
}

impl CyclingStream {
    /// Create a new cycling stream.
    ///
    /// `connect_count` is shared so tests can verify how many times the
    /// factory created a new connection (i.e. reconnections after rotation).
    pub fn new(events: Vec<MarketEvent>, delay: Duration, connect_count: Arc<AtomicU32>) -> Self {
        Self {
            events,
            index: 0,
            delay,
            connect_count,
        }
    }
}

#[async_trait]
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
// ScriptedStream — pre-loaded results for error/reconnect testing
// ---------------------------------------------------------------------------

/// A mock stream with scripted connect/subscribe results and a fixed event queue.
///
/// Useful for testing reconnection logic, error handling, and retry behavior.
/// Each call to `connect()` or `subscribe()` pops the next result from the
/// corresponding queue (defaults to `Ok(())` when exhausted).
pub struct ScriptedStream {
    connect_results: VecDeque<Result<()>>,
    subscribe_results: VecDeque<Result<()>>,
    events: VecDeque<Option<MarketEvent>>,
    connect_count: Arc<AtomicU32>,
    subscribe_count: Arc<AtomicU32>,
}

impl ScriptedStream {
    pub fn new() -> Self {
        Self {
            connect_results: VecDeque::new(),
            subscribe_results: VecDeque::new(),
            events: VecDeque::new(),
            connect_count: Arc::new(AtomicU32::new(0)),
            subscribe_count: Arc::new(AtomicU32::new(0)),
        }
    }

    pub fn with_connect_results(mut self, results: Vec<Result<()>>) -> Self {
        self.connect_results = results.into();
        self
    }

    pub fn with_subscribe_results(mut self, results: Vec<Result<()>>) -> Self {
        self.subscribe_results = results.into();
        self
    }

    pub fn with_events(mut self, events: Vec<Option<MarketEvent>>) -> Self {
        self.events = events.into();
        self
    }

    /// Get shared counters for asserting connect/subscribe call counts.
    pub fn counts(&self) -> (Arc<AtomicU32>, Arc<AtomicU32>) {
        (self.connect_count.clone(), self.subscribe_count.clone())
    }
}

#[async_trait]
impl MarketDataStream for ScriptedStream {
    async fn connect(&mut self) -> Result<()> {
        self.connect_count.fetch_add(1, Ordering::SeqCst);
        self.connect_results.pop_front().unwrap_or(Ok(()))
    }

    async fn subscribe(&mut self, _token_ids: &[TokenId]) -> Result<()> {
        self.subscribe_count.fetch_add(1, Ordering::SeqCst);
        self.subscribe_results.pop_front().unwrap_or(Ok(()))
    }

    async fn next_event(&mut self) -> Option<MarketEvent> {
        self.events.pop_front().flatten()
    }

    fn exchange_name(&self) -> &'static str {
        "mock"
    }
}
