//! Risk check types for trade validation.
//!
//! Defines result types for risk management decisions. The actual risk
//! management logic lives in the adapter layer (`adapter::risk::RiskManager`).

use crate::error::RiskError;

/// Result of a risk check for a proposed trade.
///
/// Indicates whether a trade should proceed or be rejected based on risk
/// management rules.
#[derive(Debug, Clone)]
pub enum RiskCheckResult {
    /// Trade passes all risk checks and may proceed.
    Approved,

    /// Trade is rejected due to a risk limit violation.
    Rejected(RiskError),
}

impl RiskCheckResult {
    /// Return `true` if the trade is approved.
    #[must_use]
    pub const fn is_approved(&self) -> bool {
        matches!(self, RiskCheckResult::Approved)
    }

    /// Return the rejection error if the trade was rejected.
    ///
    /// Returns `None` if the trade was approved.
    #[must_use]
    pub const fn rejection_error(&self) -> Option<&RiskError> {
        match self {
            RiskCheckResult::Rejected(e) => Some(e),
            RiskCheckResult::Approved => None,
        }
    }
}
