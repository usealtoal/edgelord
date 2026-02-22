//! Risk port for trade validation.
//!
//! This module defines the trait for risk management checks
//! that must pass before executing trades.

use crate::domain::Opportunity;
use crate::error::RiskError;

/// Result of a risk check.
#[derive(Debug, Clone)]
pub enum RiskCheckResult {
    /// Trade is allowed to proceed.
    Approved,
    /// Trade is rejected with reason.
    Rejected(RiskError),
}

impl RiskCheckResult {
    /// Check if approved.
    #[must_use]
    pub const fn is_approved(&self) -> bool {
        matches!(self, RiskCheckResult::Approved)
    }

    /// Get rejection error if rejected.
    #[must_use]
    pub const fn rejection_error(&self) -> Option<&RiskError> {
        match self {
            RiskCheckResult::Rejected(e) => Some(e),
            RiskCheckResult::Approved => None,
        }
    }
}

/// Risk gate that validates trades before execution.
///
/// The risk gate performs various checks including:
/// - Circuit breaker status
/// - Profit threshold validation
/// - Position limits per market
/// - Total exposure limits
///
/// # Implementation Notes
///
/// - Implementations must be thread-safe (`Send + Sync`)
/// - The `check` method should be fast (no I/O)
/// - On approval, exposure should be atomically reserved
/// - Failed checks should return descriptive `RiskError` variants
pub trait RiskGate: Send + Sync {
    /// Check if an opportunity passes all risk checks.
    ///
    /// On approval, atomically reserves exposure to prevent concurrent
    /// opportunities from exceeding the limit.
    ///
    /// # Arguments
    ///
    /// * `opportunity` - The opportunity to validate
    ///
    /// # Returns
    ///
    /// `RiskCheckResult::Approved` if all checks pass, or
    /// `RiskCheckResult::Rejected` with the specific failure reason.
    fn check(&self, opportunity: &Opportunity) -> RiskCheckResult;

    /// Release reserved exposure after execution completes or fails.
    ///
    /// This should be called after execution to free up the reserved
    /// exposure capacity.
    ///
    /// # Arguments
    ///
    /// * `opportunity` - The opportunity whose exposure to release
    fn release_exposure(&self, opportunity: &Opportunity);

    /// Trigger the circuit breaker.
    ///
    /// When triggered, all subsequent `check` calls will return
    /// `RiskCheckResult::Rejected` until `reset_circuit_breaker` is called.
    ///
    /// # Arguments
    ///
    /// * `reason` - Human-readable reason for triggering
    fn trigger_circuit_breaker(&self, reason: impl Into<String>);

    /// Reset the circuit breaker.
    ///
    /// Allows trading to resume after the issue has been resolved.
    fn reset_circuit_breaker(&self);

    /// Check if the circuit breaker is currently active.
    fn is_circuit_breaker_active(&self) -> bool;
}
