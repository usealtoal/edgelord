//! Execution result types for arbitrage trades.
//!
//! These types represent the outcomes of executing multi-leg arbitrage
//! opportunities. They are exchange-agnostic domain types.

use super::TokenId;

/// Unique identifier for an order.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrderId(pub String);

impl OrderId {
    /// Create a new order ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the order ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for OrderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for OrderId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for OrderId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// A successfully executed leg in an arbitrage trade.
#[derive(Debug, Clone)]
pub struct FilledLeg {
    /// Token ID for this leg.
    pub token_id: TokenId,
    /// Order ID returned by exchange.
    pub order_id: String,
}

impl FilledLeg {
    /// Create a new filled leg.
    pub fn new(token_id: TokenId, order_id: impl Into<String>) -> Self {
        Self {
            token_id,
            order_id: order_id.into(),
        }
    }
}

/// A failed leg in an arbitrage trade.
#[derive(Debug, Clone)]
pub struct FailedLeg {
    /// Token ID for this leg.
    pub token_id: TokenId,
    /// Error message.
    pub error: String,
}

impl FailedLeg {
    /// Create a new failed leg.
    pub fn new(token_id: TokenId, error: impl Into<String>) -> Self {
        Self {
            token_id,
            error: error.into(),
        }
    }
}

/// Result of executing a multi-leg arbitrage opportunity.
#[derive(Debug, Clone)]
pub enum ArbitrageExecutionResult {
    /// All legs executed successfully.
    Success { filled: Vec<FilledLeg> },
    /// Some legs executed, some failed.
    PartialFill {
        filled: Vec<FilledLeg>,
        failed: Vec<FailedLeg>,
    },
    /// All legs failed.
    Failed { reason: String },
}

impl ArbitrageExecutionResult {
    /// Check if all legs were successful.
    #[must_use]
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// Check if there was a partial fill.
    #[must_use]
    pub const fn is_partial(&self) -> bool {
        matches!(self, Self::PartialFill { .. })
    }

    /// Check if all legs failed.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// Get filled legs if any.
    #[must_use]
    pub fn filled(&self) -> &[FilledLeg] {
        match self {
            Self::Success { filled } => filled,
            Self::PartialFill { filled, .. } => filled,
            Self::Failed { .. } => &[],
        }
    }

    /// Get failed legs if any.
    #[must_use]
    pub fn failed(&self) -> &[FailedLeg] {
        match self {
            Self::Success { .. } => &[],
            Self::PartialFill { failed, .. } => failed,
            Self::Failed { .. } => &[],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_id_new_and_as_str() {
        let id = OrderId::new("order-123");
        assert_eq!(id.as_str(), "order-123");
    }

    #[test]
    fn order_id_display() {
        let id = OrderId::new("order-456");
        assert_eq!(format!("{}", id), "order-456");
    }

    #[test]
    fn filled_leg_new() {
        let leg = FilledLeg::new(TokenId::from("token-1"), "order-1");
        assert_eq!(leg.token_id.as_str(), "token-1");
        assert_eq!(leg.order_id, "order-1");
    }

    #[test]
    fn failed_leg_new() {
        let leg = FailedLeg::new(TokenId::from("token-2"), "insufficient funds");
        assert_eq!(leg.token_id.as_str(), "token-2");
        assert_eq!(leg.error, "insufficient funds");
    }

    #[test]
    fn execution_result_success() {
        let result = ArbitrageExecutionResult::Success {
            filled: vec![FilledLeg::new(TokenId::from("t1"), "o1")],
        };
        assert!(result.is_success());
        assert!(!result.is_partial());
        assert!(!result.is_failed());
        assert_eq!(result.filled().len(), 1);
        assert!(result.failed().is_empty());
    }

    #[test]
    fn execution_result_partial() {
        let result = ArbitrageExecutionResult::PartialFill {
            filled: vec![FilledLeg::new(TokenId::from("t1"), "o1")],
            failed: vec![FailedLeg::new(TokenId::from("t2"), "error")],
        };
        assert!(!result.is_success());
        assert!(result.is_partial());
        assert!(!result.is_failed());
        assert_eq!(result.filled().len(), 1);
        assert_eq!(result.failed().len(), 1);
    }

    #[test]
    fn execution_result_failed() {
        let result = ArbitrageExecutionResult::Failed {
            reason: "all legs failed".to_string(),
        };
        assert!(!result.is_success());
        assert!(!result.is_partial());
        assert!(result.is_failed());
        assert!(result.filled().is_empty());
        assert!(result.failed().is_empty());
    }
}
