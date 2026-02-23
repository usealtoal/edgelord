//! Notification port for system event dispatch.
//!
//! Defines traits and types for sending notifications about system events
//! such as detected opportunities, trade executions, risk rejections, and
//! circuit breaker state changes.
//!
//! # Overview
//!
//! - [`Notifier`]: Core notification trait
//! - [`NotifierRegistry`]: Composite notifier for broadcasting to multiple handlers
//! - [`Event`]: Enum of all notification event types

use rust_decimal::Decimal;

use crate::domain::{opportunity::Opportunity, trade::TradeResult};
use crate::error::RiskError;

/// System event that triggers a notification.
///
/// Represents all types of events that can be sent to notification handlers.
#[derive(Debug, Clone)]
pub enum Event {
    /// Arbitrage opportunity detected and ready for execution.
    OpportunityDetected(OpportunityEvent),

    /// Trade execution completed (success, partial, or failure).
    ExecutionCompleted(ExecutionEvent),

    /// Risk check rejected a proposed trade.
    RiskRejected(RiskEvent),

    /// Circuit breaker activated, halting trading.
    CircuitBreakerActivated {
        /// Human-readable description of why the circuit breaker was triggered.
        reason: String,
    },

    /// Circuit breaker reset, resuming trading.
    CircuitBreakerReset,

    /// Daily trading summary.
    DailySummary(SummaryEvent),

    /// Market relations discovered by LLM inference.
    RelationsDiscovered(RelationsEvent),
}

/// Event data for a detected arbitrage opportunity.
#[derive(Debug, Clone)]
pub struct OpportunityEvent {
    /// Identifier of the market where the opportunity was found.
    pub market_id: String,

    /// Human-readable market question.
    pub question: String,

    /// Arbitrage edge (payout minus cost).
    pub edge: Decimal,

    /// Trade volume in shares.
    pub volume: Decimal,

    /// Expected profit from executing this opportunity.
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

/// Event data for a completed trade execution.
#[derive(Debug, Clone)]
pub struct ExecutionEvent {
    /// Identifier of the market where the trade was executed.
    pub market_id: String,

    /// Whether the execution was fully successful.
    pub success: bool,

    /// Human-readable execution details or error message.
    pub details: String,
}

impl ExecutionEvent {
    /// Create an execution event from an arbitrage trade result.
    ///
    /// # Arguments
    ///
    /// * `market_id` - Identifier of the market where the trade was executed.
    /// * `result` - Outcome of the trade execution.
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

/// Event data for a risk-rejected trade.
#[derive(Debug, Clone)]
pub struct RiskEvent {
    /// Identifier of the market for the rejected trade.
    pub market_id: String,

    /// Human-readable description of why the trade was rejected.
    pub reason: String,
}

impl RiskEvent {
    /// Create a risk event from a market identifier and risk error.
    ///
    /// # Arguments
    ///
    /// * `market_id` - Identifier of the market for the rejected trade.
    /// * `error` - Risk error describing the rejection reason.
    #[must_use]
    pub fn new(market_id: &str, error: &RiskError) -> Self {
        Self {
            market_id: market_id.to_string(),
            reason: error.to_string(),
        }
    }
}

/// Event data for a daily trading summary.
#[derive(Debug, Clone)]
pub struct SummaryEvent {
    /// Date covered by this summary.
    pub date: chrono::NaiveDate,

    /// Total number of opportunities detected.
    pub opportunities_detected: u64,

    /// Total number of trades executed (including partial and failed).
    pub trades_executed: u64,

    /// Number of fully successful trades.
    pub trades_successful: u64,

    /// Total realized profit for the day.
    pub total_profit: Decimal,

    /// Current open exposure amount.
    pub current_exposure: Decimal,
}

/// Event data for discovered market relations.
#[derive(Debug, Clone)]
pub struct RelationsEvent {
    /// Number of relations discovered in this inference batch.
    pub relations_count: usize,

    /// Details of each discovered relation.
    pub relations: Vec<RelationDetail>,
}

/// Detail of a single discovered market relation.
#[derive(Debug, Clone)]
pub struct RelationDetail {
    /// Type of logical relation (e.g., "mutually_exclusive", "implies", "exactly_one").
    pub relation_type: String,

    /// Confidence score from the inference model (0.0 to 1.0).
    pub confidence: f64,

    /// Market questions involved in this relation.
    pub market_questions: Vec<String>,

    /// LLM reasoning explaining why this relation exists.
    pub reasoning: String,
}

/// Handler for system event notifications.
///
/// Implement this trait to receive events from the trading system.
/// Notifications are fire-and-forget; the caller does not await completion.
///
/// # Thread Safety
///
/// Implementations must be thread-safe (`Send + Sync`).
///
/// # Implementation Notes
///
/// - The [`notify`](Self::notify) method should return quickly
/// - For slow operations (HTTP calls, database writes), spawn an async task
/// - Failures should be logged rather than propagated
pub trait Notifier: Send + Sync {
    /// Handle a system event.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to handle.
    ///
    /// This method should return quickly. For slow operations such as HTTP
    /// requests or database writes, implementations should spawn an async task.
    fn notify(&self, event: Event);
}

/// Composite notifier that broadcasts events to multiple handlers.
///
/// Implements the composite pattern to allow registering multiple notifiers
/// and broadcasting events to all of them.
pub struct NotifierRegistry {
    /// Registered notifier handlers.
    notifiers: Vec<Box<dyn Notifier>>,
}

impl NotifierRegistry {
    /// Create an empty notifier registry.
    #[must_use]
    pub fn new() -> Self {
        Self { notifiers: vec![] }
    }

    /// Register a notifier to receive events.
    ///
    /// # Arguments
    ///
    /// * `notifier` - Notifier implementation to register.
    pub fn register(&mut self, notifier: Box<dyn Notifier>) {
        self.notifiers.push(notifier);
    }

    /// Broadcast an event to all registered notifiers.
    ///
    /// # Arguments
    ///
    /// * `event` - Event to send to all notifiers.
    ///
    /// Each notifier receives a clone of the event.
    pub fn notify_all(&self, event: Event) {
        for notifier in &self.notifiers {
            notifier.notify(event.clone());
        }
    }

    /// Return the number of registered notifiers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.notifiers.len()
    }

    /// Return `true` if no notifiers are registered.
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

/// No-op notifier that discards all events.
///
/// Useful for testing or when notifications are disabled.
pub struct NullNotifier;

impl Notifier for NullNotifier {
    fn notify(&self, _event: Event) {}
}

/// Notifier that logs events using the `tracing` framework.
///
/// Useful for debugging and as a fallback when other notification
/// channels are unavailable.
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
