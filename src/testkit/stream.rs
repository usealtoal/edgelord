//! Mock [`MarketDataStream`] implementations for testing.
//!
//! Three mock stream types for different testing needs:
//!
//! - [`ScriptedStream`] — Pre-loaded connect/subscribe results and events.
//!   Best for: error handling, reconnection logic, retry behavior.
//!
//! - [`CyclingStream`] — Infinite event loop with configurable delay.
//!   Best for: timing-based tests (TTL rotation, health monitoring, backpressure).
//!
//! - [`ChannelStream`] — Channel-backed stream with external control handle.
//!   Best for: integration tests needing precise, on-demand event delivery.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;

use crate::domain::TokenId;
use crate::error::Result;
use crate::runtime::exchange::{MarketDataStream, MarketEvent};

// ---------------------------------------------------------------------------
// ScriptedStream
// ---------------------------------------------------------------------------

/// A mock stream with scripted connect/subscribe results and a fixed event queue.
///
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

    /// Replace the connect counter with a shared one.
    ///
    /// Useful when a factory creates multiple streams that should share
    /// a single counter (e.g. counting total reconnections across rotations).
    pub fn set_connect_count(&mut self, counter: Arc<AtomicU32>) {
        self.connect_count = counter;
    }

    pub fn connect_count(&self) -> u32 {
        self.connect_count.load(Ordering::SeqCst)
    }

    pub fn subscribe_count(&self) -> u32 {
        self.subscribe_count.load(Ordering::SeqCst)
    }
}

impl Default for ScriptedStream {
    fn default() -> Self {
        Self::new()
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

// ---------------------------------------------------------------------------
// CyclingStream
// ---------------------------------------------------------------------------

/// A mock stream that yields events from a fixed list in an infinite loop.
///
/// Each call to `next_event()` sleeps for `delay` before returning. If the
/// event list is empty, blocks forever (simulates a quiet connection).
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
    /// factory created a new connection (e.g. after TTL rotation).
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
// OneEventThenSilent
// ---------------------------------------------------------------------------

/// A mock stream that delivers one event then blocks forever.
///
/// The connection task stays alive (not crashed), but produces no further
/// events — ideal for testing silent death detection.
pub struct OneEventThenSilentStream {
    event: Option<MarketEvent>,
    connect_count: Arc<AtomicU32>,
}

impl OneEventThenSilentStream {
    pub fn new(event: MarketEvent, connect_count: Arc<AtomicU32>) -> Self {
        Self {
            event: Some(event),
            connect_count,
        }
    }
}

#[async_trait]
impl MarketDataStream for OneEventThenSilentStream {
    async fn connect(&mut self) -> Result<()> {
        self.connect_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn subscribe(&mut self, _: &[TokenId]) -> Result<()> {
        Ok(())
    }

    async fn next_event(&mut self) -> Option<MarketEvent> {
        if let Some(event) = self.event.take() {
            return Some(event);
        }
        // Block forever — connection stays alive but silent.
        std::future::pending().await
    }

    fn exchange_name(&self) -> &'static str {
        "mock"
    }
}

// ---------------------------------------------------------------------------
// ChannelStream
// ---------------------------------------------------------------------------

/// A mock stream controlled externally via a [`ChannelStreamHandle`].
///
/// Events are sent into the handle's `event_tx` and read by the consumer
/// via `next_event()`. No real network I/O.
pub struct ChannelStream {
    event_rx: tokio::sync::mpsc::Receiver<Option<MarketEvent>>,
    connect_count: Arc<AtomicU32>,
    subscribe_count: Arc<AtomicU32>,
    subscribed_tokens: Arc<Mutex<Vec<TokenId>>>,
}

/// Control handle for a [`ChannelStream`].
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

    /// Get a cloned sender for sending events without holding a reference.
    ///
    /// Useful in async tests where you need to avoid holding a `MutexGuard`
    /// across await points.
    pub fn sender(&self) -> tokio::sync::mpsc::Sender<Option<MarketEvent>> {
        self.event_tx.clone()
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
