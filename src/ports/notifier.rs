//! Notifier port for event notifications.
//!
//! This module defines the trait for sending notifications about
//! system events such as detected opportunities, executions, and alerts.

use rust_decimal::Decimal;

use crate::domain::{ArbitrageExecutionResult, Opportunity};
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
    /// Market relations discovered by inference.
    RelationsDiscovered(RelationsEvent),
}

/// Opportunity detection event.
#[derive(Debug, Clone)]
pub struct OpportunityEvent {
    pub market_id: String,
    pub question: String,
    pub edge: Decimal,
    pub volume: Decimal,
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
    pub total_profit: Decimal,
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
