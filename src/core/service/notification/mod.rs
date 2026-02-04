//! Notification system for alerts and events.
//!
//! The `Notifier` trait defines the interface for notification handlers.
//! Multiple notifiers can be registered with the `NotifierRegistry`.

#[cfg(feature = "telegram")]
mod telegram;

#[cfg(feature = "telegram")]
pub use telegram::{TelegramConfig, TelegramNotifier};

use crate::core::domain::Opportunity;
use crate::core::exchange::ArbitrageExecutionResult;
use crate::error::RiskError;

/// Events that can trigger notifications.
#[derive(Debug, Clone)]
pub enum Event {
    /// Arbitrage opportunity detected.
    OpportunityDetected(OpportunityEvent),
    /// Execution completed (success or failure).
    ExecutionCompleted(ExecutionEvent),
    /// Risk check rejected a trade.
    RiskRejected(RiskEvent),
    /// Circuit breaker activated.
    CircuitBreakerActivated { reason: String },
    /// Circuit breaker reset.
    CircuitBreakerReset,
    /// Daily summary.
    DailySummary(SummaryEvent),
}

/// Opportunity detection event.
#[derive(Debug, Clone)]
pub struct OpportunityEvent {
    pub market_id: String,
    pub question: String,
    pub edge: rust_decimal::Decimal,
    pub volume: rust_decimal::Decimal,
    pub expected_profit: rust_decimal::Decimal,
}

impl From<&Opportunity> for OpportunityEvent {
    fn from(opp: &Opportunity) -> Self {
        Self {
            market_id: opp.market_id().to_string(),
            question: opp.question().to_string(),
            edge: opp.edge(),
            volume: opp.volume(),
            expected_profit: opp.expected_profit(),
        }
    }
}

/// Execution result event.
#[derive(Debug, Clone)]
pub struct ExecutionEvent {
    pub market_id: String,
    pub success: bool,
    pub details: String,
}

impl ExecutionEvent {
    #[must_use]
    pub fn from_result(market_id: &str, result: &ArbitrageExecutionResult) -> Self {
        match result {
            ArbitrageExecutionResult::Success { filled } => {
                let order_ids: Vec<_> = filled.iter().map(|f| f.order_id.as_str()).collect();
                Self {
                    market_id: market_id.to_string(),
                    success: true,
                    details: format!("Orders: {}", order_ids.join(", ")),
                }
            }
            ArbitrageExecutionResult::PartialFill { filled, failed } => {
                let filled_ids: Vec<_> = filled.iter().map(|f| f.token_id.to_string()).collect();
                let failed_ids: Vec<_> = failed.iter().map(|f| f.token_id.to_string()).collect();
                Self {
                    market_id: market_id.to_string(),
                    success: false,
                    details: format!(
                        "Partial fill - filled: {:?}, failed: {:?}",
                        filled_ids, failed_ids
                    ),
                }
            }
            ArbitrageExecutionResult::Failed { reason } => Self {
                market_id: market_id.to_string(),
                success: false,
                details: format!("Failed: {reason}"),
            },
        }
    }
}

/// Risk rejection event.
#[derive(Debug, Clone)]
pub struct RiskEvent {
    pub market_id: String,
    pub reason: String,
}

impl RiskEvent {
    #[must_use]
    pub fn new(market_id: &str, error: &RiskError) -> Self {
        Self {
            market_id: market_id.to_string(),
            reason: error.to_string(),
        }
    }
}

/// Daily summary event.
#[derive(Debug, Clone)]
pub struct SummaryEvent {
    pub date: chrono::NaiveDate,
    pub opportunities_detected: u64,
    pub trades_executed: u64,
    pub trades_successful: u64,
    pub total_profit: rust_decimal::Decimal,
    pub current_exposure: rust_decimal::Decimal,
}

/// Trait for notification handlers.
///
/// Implement this trait to receive events from the system.
/// Notifications are fire-and-forget (async but not awaited).
pub trait Notifier: Send + Sync {
    /// Handle an event.
    fn notify(&self, event: Event);
}

/// Registry of notifiers.
pub struct NotifierRegistry {
    notifiers: Vec<Box<dyn Notifier>>,
}

impl NotifierRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self { notifiers: vec![] }
    }

    /// Register a notifier.
    pub fn register(&mut self, notifier: Box<dyn Notifier>) {
        self.notifiers.push(notifier);
    }

    /// Notify all registered notifiers.
    pub fn notify_all(&self, event: Event) {
        for notifier in &self.notifiers {
            notifier.notify(event.clone());
        }
    }

    /// Number of registered notifiers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.notifiers.len()
    }

    /// Check if registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.notifiers.is_empty()
    }
}

impl Default for NotifierRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// A no-op notifier for testing or when notifications are disabled.
pub struct NullNotifier;

impl Notifier for NullNotifier {
    fn notify(&self, _event: Event) {
        // Do nothing
    }
}

/// A logging notifier that logs events via tracing.
pub struct LogNotifier;

impl Notifier for LogNotifier {
    fn notify(&self, event: Event) {
        use tracing::info;
        match event {
            Event::OpportunityDetected(e) => {
                info!(
                    market_id = %e.market_id,
                    edge = %e.edge,
                    profit = %e.expected_profit,
                    "Opportunity detected"
                );
            }
            Event::ExecutionCompleted(e) => {
                info!(
                    market_id = %e.market_id,
                    success = e.success,
                    details = %e.details,
                    "Execution completed"
                );
            }
            Event::RiskRejected(e) => {
                info!(
                    market_id = %e.market_id,
                    reason = %e.reason,
                    "Risk rejected"
                );
            }
            Event::CircuitBreakerActivated { reason } => {
                info!(reason = %reason, "Circuit breaker activated");
            }
            Event::CircuitBreakerReset => {
                info!("Circuit breaker reset");
            }
            Event::DailySummary(e) => {
                info!(
                    date = %e.date,
                    opportunities = e.opportunities_detected,
                    trades = e.trades_executed,
                    successful = e.trades_successful,
                    profit = %e.total_profit,
                    "Daily summary"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct CountingNotifier {
        count: Arc<AtomicUsize>,
    }

    impl Notifier for CountingNotifier {
        fn notify(&self, _event: Event) {
            self.count.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn test_registry_notify_all() {
        let count = Arc::new(AtomicUsize::new(0));
        let mut registry = NotifierRegistry::new();

        registry.register(Box::new(CountingNotifier {
            count: count.clone(),
        }));
        registry.register(Box::new(CountingNotifier {
            count: count.clone(),
        }));

        registry.notify_all(Event::CircuitBreakerReset);

        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_null_notifier() {
        let notifier = NullNotifier;
        notifier.notify(Event::CircuitBreakerReset);
        // Just verify it doesn't panic
    }

    #[test]
    fn test_registry_len_and_is_empty() {
        let mut registry = NotifierRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);

        registry.register(Box::new(NullNotifier));
        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_registry_default() {
        let registry = NotifierRegistry::default();
        assert!(registry.is_empty());
    }
}
