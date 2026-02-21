//! Reconnecting wrapper for MarketDataStream.
//!
//! Provides automatic reconnection with exponential backoff and circuit breaker
//! for any MarketDataStream implementation.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use crate::domain::TokenId;
use crate::error::Error;
use crate::runtime::ReconnectionConfig;

use super::{MarketDataStream, MarketEvent};

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
        let base_delay = Duration::from_millis(self.current_delay_ms);
        let jitter_ms = self.jitter_ms(base_delay);
        let delay = base_delay + Duration::from_millis(jitter_ms);

        // Increase delay for next attempt
        let next_delay = (self.current_delay_ms as f64 * self.config.backoff_multiplier) as u64;
        self.current_delay_ms = next_delay.min(self.config.max_delay_ms);

        delay
    }

    fn jitter_ms(&self, base_delay: Duration) -> u64 {
        let jitter_range_ms = (base_delay.as_millis() as u64) / 5;
        if jitter_range_ms == 0 {
            return 0;
        }

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        (nanos as u64) % (jitter_range_ms + 1)
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
                    debug!(
                        tokens = self.subscribed_tokens.len(),
                        "Resubscribing to tokens"
                    );
                    if let Err(err) = self.inner.subscribe(&self.subscribed_tokens).await {
                        error!(error = %err, "Resubscribe failed after reconnect");
                        self.connected = false;
                        self.record_failure();
                        return Err(err);
                    }
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
    use std::sync::atomic::Ordering;

    use crate::testkit;
    use crate::testkit::stream::ScriptedStream;

    /// Reconnection config with non-zero delays for testing backoff behavior.
    fn backoff_config() -> ReconnectionConfig {
        ReconnectionConfig {
            initial_delay_ms: 10,
            max_delay_ms: 100,
            backoff_multiplier: 2.0,
            max_consecutive_failures: 3,
            circuit_breaker_cooldown_ms: 50,
        }
    }

    #[tokio::test]
    async fn test_successful_connection() {
        let mock = ScriptedStream::new()
            .with_events(vec![Some(testkit::domain::snapshot_event("token1"))]);

        let mut stream = ReconnectingDataStream::new(mock, backoff_config());
        stream.connect().await.unwrap();

        let event = stream.next_event().await;
        assert!(matches!(event, Some(MarketEvent::OrderBookSnapshot { .. })));
    }

    #[tokio::test]
    async fn test_reconnect_after_disconnect() {
        let mock = ScriptedStream::new().with_events(vec![
            Some(testkit::domain::disconnect_event("test")),
            Some(testkit::domain::snapshot_event("token1")),
        ]);
        let (connect_count, subscribe_count) = mock.counts();

        let mut stream = ReconnectingDataStream::new(mock, backoff_config());
        stream.connect().await.unwrap();
        stream
            .subscribe(&[testkit::domain::token("token1")])
            .await
            .unwrap();

        // First call triggers reconnect, second returns snapshot
        let event = stream.next_event().await;
        assert!(matches!(event, Some(MarketEvent::OrderBookSnapshot { .. })));

        assert!(connect_count.load(Ordering::SeqCst) >= 2);
        assert!(subscribe_count.load(Ordering::SeqCst) >= 2);
    }

    #[tokio::test]
    async fn test_exponential_backoff() {
        let mut stream = ReconnectingDataStream::new(ScriptedStream::new(), backoff_config());

        let assert_delay_in_range = |delay: Duration, base_ms: u64| {
            let max_ms = base_ms + (base_ms / 5);
            assert!(
                (base_ms..=max_ms).contains(&(delay.as_millis() as u64)),
                "delay {delay:?} not within {base_ms}..={max_ms} ms"
            );
        };

        assert_delay_in_range(stream.next_delay(), 10);
        assert_delay_in_range(stream.next_delay(), 20);
        assert_delay_in_range(stream.next_delay(), 40);
        assert_delay_in_range(stream.next_delay(), 80);
        assert_delay_in_range(stream.next_delay(), 100); // Capped at max
    }

    #[tokio::test]
    async fn test_circuit_breaker_trips() {
        let mut stream = ReconnectingDataStream::new(ScriptedStream::new(), backoff_config());

        for _ in 0..3 {
            stream.record_failure();
        }

        assert!(matches!(stream.circuit_state, CircuitState::Open { .. }));
        assert!(!stream.circuit_allows_connection());
    }

    #[tokio::test]
    async fn test_reset_backoff() {
        let mut stream = ReconnectingDataStream::new(ScriptedStream::new(), backoff_config());

        stream.consecutive_failures = 5;
        stream.current_delay_ms = 1000;
        stream.reset_backoff();

        assert_eq!(stream.consecutive_failures, 0);
        assert_eq!(stream.current_delay_ms, 10);
        assert!(matches!(stream.circuit_state, CircuitState::Closed));
    }
}
