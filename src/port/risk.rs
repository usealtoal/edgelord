//! Risk types for trade validation.
//!
//! Defines the result type for risk checks. The actual risk management
//! implementation lives in `adapter::risk::RiskManager`.

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
