//! Notifier port for event notifications.
//!
//! This module defines the trait for sending notifications about
//! system events such as detected opportunities, executions, and alerts.

use rust_decimal::Decimal;

use crate::domain::{opportunity::Opportunity, trade::TradeResult};
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
    CircuitBreakerActivated {
        /// The reason for activation.
        reason: String,
    },
    /// Circuit breaker reset.
    CircuitBreakerReset,
    /// Daily summary.
    DailySummary(SummaryEvent),
    /// Market relations discovered by inference.
    RelationsDiscovered(RelationsEvent),
}

/// Opportunity detection event.
#[derive(Debug, Clone)]
pub struct OpportunityEvent {
    /// The market ID where the opportunity was found.
    pub market_id: String,
    /// The market question.
    pub question: String,
    /// The arbitrage edge (payout - cost).
    pub edge: Decimal,
    /// The trade volume.
    pub volume: Decimal,
    /// Expected profit from the opportunity.
    pub expected_profit: Decimal,
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
    /// The market ID for the executed trade.
    pub market_id: String,
    /// Whether the execution was successful.
    pub success: bool,
    /// Additional execution details.
    pub details: String,
}

impl ExecutionEvent {
    /// Create an execution event from an arbitrage execution result.
    #[must_use]
    pub fn from_result(market_id: &str, result: &TradeResult) -> Self {
        match result {
            TradeResult::Success { fills } => {
                let order_ids: Vec<_> = fills.iter().map(|f| f.order_id.as_str()).collect();
                Self {
                    market_id: market_id.to_string(),
                    success: true,
                    details: format!("Orders: {}", order_ids.join(", ")),
                }
            }
            TradeResult::Partial { fills, failures } => {
                let fill_ids: Vec<_> = fills.iter().map(|f| f.token_id.to_string()).collect();
                let failure_ids: Vec<_> = failures.iter().map(|f| f.token_id.to_string()).collect();
                Self {
                    market_id: market_id.to_string(),
                    success: false,
                    details: format!(
                        "Partial fill - fills: {:?}, failures: {:?}",
                        fill_ids, failure_ids
                    ),
                }
            }
            TradeResult::Failed { reason } => Self {
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
    /// The market ID for the rejected trade.
    pub market_id: String,
    /// The rejection reason.
    pub reason: String,
}

impl RiskEvent {
    /// Create a new risk event from a market ID and risk error.
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
    /// The date for this summary.
    pub date: chrono::NaiveDate,
    /// Total opportunities detected.
    pub opportunities_detected: u64,
    /// Total trades executed.
    pub trades_executed: u64,
    /// Number of successful trades.
    pub trades_successful: u64,
    /// Total profit for the day.
    pub total_profit: Decimal,
    /// Current exposure amount.
    pub current_exposure: Decimal,
}

/// Market relations discovered event.
#[derive(Debug, Clone)]
pub struct RelationsEvent {
    /// Number of relations discovered in this batch.
    pub relations_count: usize,
    /// Details of each discovered relation.
    pub relations: Vec<RelationDetail>,
}

/// Detail of a single discovered relation.
#[derive(Debug, Clone)]
pub struct RelationDetail {
    /// Type of relation (e.g., "mutually_exclusive", "implies", "exactly_one").
    pub relation_type: String,
    /// Confidence score (0.0-1.0).
    pub confidence: f64,
    /// Market questions involved.
    pub market_questions: Vec<String>,
    /// LLM reasoning for this relation.
    pub reasoning: String,
}

/// Trait for notification handlers.
///
/// Implement this trait to receive events from the system.
/// Notifications are fire-and-forget (async but not awaited).
///
/// # Implementation Notes
///
/// - Implementations must be thread-safe (`Send + Sync`)
/// - The `notify` method should not block or perform slow I/O synchronously
/// - Consider spawning async tasks for slow operations
pub trait Notifier: Send + Sync {
    /// Handle an event.
    ///
    /// This method should return quickly. For slow operations (e.g., HTTP calls),
    /// implementations should spawn an async task.
    fn notify(&self, event: Event);
}

/// Registry of notifiers (composite pattern).
///
/// Broadcasts events to all registered notifiers.
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
