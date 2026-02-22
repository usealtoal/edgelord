//! Notification adapters.
//!
//! Implements the `port::Notifier` trait for various notification backends.

#[cfg(feature = "telegram")]
mod telegram;

#[cfg(feature = "telegram")]
pub use telegram::{RuntimeStats, TelegramConfig, TelegramNotifier};

use crate::port::{Event, Notifier};

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
    fn notify(&self, _event: Event) {}
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
            Event::RelationsDiscovered(e) => {
                info!(relations = e.relations_count, "Relations discovered");
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
}
