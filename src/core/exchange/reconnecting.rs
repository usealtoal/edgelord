//! Reconnecting wrapper for MarketDataStream.
//!
//! Provides automatic reconnection with exponential backoff and circuit breaker
//! for any MarketDataStream implementation.

use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use crate::app::ReconnectionConfig;
use crate::core::domain::TokenId;
use crate::core::exchange::{MarketDataStream, MarketEvent};
use crate::error::Error;

/// Circuit breaker state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CircuitState {
    /// Normal operation, connections allowed.
    Closed,
    /// Too many failures, blocking connections temporarily.
    Open { until: Instant },
}

/// Wrapper that adds reconnection logic to any MarketDataStream.
pub struct ReconnectingDataStream<S: MarketDataStream> {
    /// The underlying data stream.
    inner: S,
    /// Reconnection configuration.
    config: ReconnectionConfig,
    /// Token IDs to resubscribe after reconnection.
    subscribed_tokens: Vec<TokenId>,
    /// Current consecutive failure count.
    consecutive_failures: u32,
    /// Current backoff delay.
    current_delay_ms: u64,
    /// Circuit breaker state.
    circuit_state: CircuitState,
    /// Whether we're currently connected.
    connected: bool,
}

impl<S: MarketDataStream> ReconnectingDataStream<S> {
    /// Create a new reconnecting wrapper.
    pub fn new(inner: S, config: ReconnectionConfig) -> Self {
        let initial_delay = config.initial_delay_ms;
        Self {
            inner,
            config,
            subscribed_tokens: Vec::new(),
            consecutive_failures: 0,
            current_delay_ms: initial_delay,
            circuit_state: CircuitState::Closed,
            connected: false,
        }
    }

    /// Reset backoff state after successful connection.
    fn reset_backoff(&mut self) {
        self.consecutive_failures = 0;
        self.current_delay_ms = self.config.initial_delay_ms;
        self.circuit_state = CircuitState::Closed;
    }

    /// Calculate next backoff delay using exponential backoff.
    fn next_delay(&mut self) -> Duration {
        let delay = Duration::from_millis(self.current_delay_ms);

        // Increase delay for next attempt
        let next_delay = (self.current_delay_ms as f64 * self.config.backoff_multiplier) as u64;
        self.current_delay_ms = next_delay.min(self.config.max_delay_ms);

        delay
    }

    /// Check if circuit breaker allows connection attempts.
    fn circuit_allows_connection(&mut self) -> bool {
        match self.circuit_state {
            CircuitState::Closed => true,
            CircuitState::Open { until } => {
                if Instant::now() >= until {
                    info!("Circuit breaker cooldown expired, allowing reconnection");
                    self.circuit_state = CircuitState::Closed;
                    self.reset_backoff();
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Record a connection failure and possibly trip circuit breaker.
    fn record_failure(&mut self) {
        self.consecutive_failures += 1;
        self.connected = false;

        if self.consecutive_failures >= self.config.max_consecutive_failures {
            let cooldown = Duration::from_millis(self.config.circuit_breaker_cooldown_ms);
            let until = Instant::now() + cooldown;
            self.circuit_state = CircuitState::Open { until };
            error!(
                failures = self.consecutive_failures,
                cooldown_secs = cooldown.as_secs(),
                "Circuit breaker tripped, pausing reconnection attempts"
            );
        }
    }

    /// Attempt to reconnect with backoff.
    async fn reconnect(&mut self) -> Result<(), Error> {
        if !self.circuit_allows_connection() {
            // Circuit is open, wait for cooldown
            if let CircuitState::Open { until } = self.circuit_state {
                let remaining = until.saturating_duration_since(Instant::now());
                warn!(
                    remaining_secs = remaining.as_secs(),
                    "Circuit breaker open, waiting for cooldown"
                );
                sleep(remaining).await;
                // After cooldown, circuit should allow connection
                self.circuit_state = CircuitState::Closed;
                self.reset_backoff();
            }
        }

        let delay = self.next_delay();
        info!(
            delay_ms = delay.as_millis(),
            attempt = self.consecutive_failures + 1,
            "Reconnecting after delay"
        );
        sleep(delay).await;

        match self.inner.connect().await {
            Ok(()) => {
                info!("Reconnected successfully");
                self.connected = true;

                // Resubscribe to tokens
                if !self.subscribed_tokens.is_empty() {
                    debug!(tokens = self.subscribed_tokens.len(), "Resubscribing to tokens");
                    self.inner.subscribe(&self.subscribed_tokens).await?;
                }

                self.reset_backoff();
                Ok(())
            }
            Err(e) => {
                error!(error = %e, "Reconnection failed");
                self.record_failure();
                Err(e)
            }
        }
    }
}

#[async_trait]
impl<S: MarketDataStream + Send> MarketDataStream for ReconnectingDataStream<S> {
    async fn connect(&mut self) -> Result<(), Error> {
        let result = self.inner.connect().await;
        if result.is_ok() {
            self.connected = true;
            self.reset_backoff();
        }
        result
    }

    async fn subscribe(&mut self, token_ids: &[TokenId]) -> Result<(), Error> {
        // Store tokens for resubscription after reconnect
        self.subscribed_tokens = token_ids.to_vec();
        self.inner.subscribe(token_ids).await
    }

    async fn next_event(&mut self) -> Option<MarketEvent> {
        loop {
            // If not connected, try to reconnect
            if !self.connected {
                if let Err(e) = self.reconnect().await {
                    warn!(error = %e, "Reconnection attempt failed, will retry");
                    continue;
                }
            }

            // Get next event from inner stream
            match self.inner.next_event().await {
                Some(MarketEvent::Disconnected { reason }) => {
                    warn!(reason = %reason, "Connection lost, will reconnect");
                    self.connected = false;
                    self.record_failure();
                    // Don't return the disconnected event, just reconnect
                    continue;
                }
                Some(event) => {
                    // Reset failure count on successful event
                    if self.consecutive_failures > 0 {
                        debug!("Received event after reconnection, resetting failure count");
                        self.reset_backoff();
                    }
                    return Some(event);
                }
                None => {
                    // Stream ended unexpectedly
                    warn!("Data stream ended unexpectedly, will reconnect");
                    self.connected = false;
                    self.record_failure();
                    continue;
                }
            }
        }
    }

    fn exchange_name(&self) -> &'static str {
        self.inner.exchange_name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    use crate::core::domain::OrderBook;

    /// Mock data stream for testing reconnection logic.
    struct MockDataStream {
        connect_results: VecDeque<Result<(), Error>>,
        events: VecDeque<Option<MarketEvent>>,
        connect_count: Arc<AtomicU32>,
        subscribe_count: Arc<AtomicU32>,
    }

    impl MockDataStream {
        fn new() -> Self {
            Self {
                connect_results: VecDeque::new(),
                events: VecDeque::new(),
                connect_count: Arc::new(AtomicU32::new(0)),
                subscribe_count: Arc::new(AtomicU32::new(0)),
            }
        }

        fn with_connect_results(mut self, results: Vec<Result<(), Error>>) -> Self {
            self.connect_results = results.into();
            self
        }

        fn with_events(mut self, events: Vec<Option<MarketEvent>>) -> Self {
            self.events = events.into();
            self
        }

        fn connect_count(&self) -> u32 {
            self.connect_count.load(Ordering::SeqCst)
        }

        fn subscribe_count(&self) -> u32 {
            self.subscribe_count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl MarketDataStream for MockDataStream {
        async fn connect(&mut self) -> Result<(), Error> {
            self.connect_count.fetch_add(1, Ordering::SeqCst);
            self.connect_results
                .pop_front()
                .unwrap_or(Ok(()))
        }

        async fn subscribe(&mut self, _token_ids: &[TokenId]) -> Result<(), Error> {
            self.subscribe_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn next_event(&mut self) -> Option<MarketEvent> {
            self.events.pop_front().flatten()
        }

        fn exchange_name(&self) -> &'static str {
            "mock"
        }
    }

    fn test_config() -> ReconnectionConfig {
        ReconnectionConfig {
            initial_delay_ms: 10, // Fast for tests
            max_delay_ms: 100,
            backoff_multiplier: 2.0,
            max_consecutive_failures: 3,
            circuit_breaker_cooldown_ms: 50,
        }
    }

    #[tokio::test]
    async fn test_successful_connection() {
        let mock = MockDataStream::new()
            .with_events(vec![
                Some(MarketEvent::OrderBookSnapshot {
                    token_id: TokenId::from("token1".to_string()),
                    book: OrderBook::new(TokenId::from("token1".to_string())),
                }),
            ]);

        let mut stream = ReconnectingDataStream::new(mock, test_config());
        stream.connect().await.unwrap();

        let event = stream.next_event().await;
        assert!(matches!(event, Some(MarketEvent::OrderBookSnapshot { .. })));
    }

    #[tokio::test]
    async fn test_reconnect_after_disconnect() {
        let mock = MockDataStream::new()
            .with_events(vec![
                Some(MarketEvent::Disconnected { reason: "test".into() }),
                Some(MarketEvent::OrderBookSnapshot {
                    token_id: TokenId::from("token1".to_string()),
                    book: OrderBook::new(TokenId::from("token1".to_string())),
                }),
            ]);

        let mut stream = ReconnectingDataStream::new(mock, test_config());
        stream.connect().await.unwrap();
        stream.subscribe(&[TokenId::from("token1".to_string())]).await.unwrap();

        // First call should trigger reconnect, second should return snapshot
        let event = stream.next_event().await;
        assert!(matches!(event, Some(MarketEvent::OrderBookSnapshot { .. })));

        // Should have reconnected (connect called twice total)
        assert!(stream.inner.connect_count() >= 2);
        // Should have resubscribed
        assert!(stream.inner.subscribe_count() >= 2);
    }

    #[tokio::test]
    async fn test_exponential_backoff() {
        let mut stream = ReconnectingDataStream::new(
            MockDataStream::new(),
            test_config(),
        );

        // Initial delay
        let delay1 = stream.next_delay();
        assert_eq!(delay1.as_millis(), 10);

        // After one failure, delay doubles
        let delay2 = stream.next_delay();
        assert_eq!(delay2.as_millis(), 20);

        // After two failures, delay doubles again
        let delay3 = stream.next_delay();
        assert_eq!(delay3.as_millis(), 40);

        // Should cap at max_delay_ms
        let delay4 = stream.next_delay();
        assert_eq!(delay4.as_millis(), 80);

        let delay5 = stream.next_delay();
        assert_eq!(delay5.as_millis(), 100); // Capped at max
    }

    #[tokio::test]
    async fn test_circuit_breaker_trips() {
        let mut stream = ReconnectingDataStream::new(
            MockDataStream::new(),
            test_config(),
        );

        // Record failures up to threshold
        for _ in 0..3 {
            stream.record_failure();
        }

        // Circuit should be open
        assert!(matches!(stream.circuit_state, CircuitState::Open { .. }));
        assert!(!stream.circuit_allows_connection());
    }

    #[tokio::test]
    async fn test_reset_backoff() {
        let mut stream = ReconnectingDataStream::new(
            MockDataStream::new(),
            test_config(),
        );

        // Simulate some failures
        stream.consecutive_failures = 5;
        stream.current_delay_ms = 1000;

        // Reset
        stream.reset_backoff();

        assert_eq!(stream.consecutive_failures, 0);
        assert_eq!(stream.current_delay_ms, 10); // Back to initial
        assert!(matches!(stream.circuit_state, CircuitState::Closed));
    }
}
