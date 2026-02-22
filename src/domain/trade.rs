//! Trade execution result types.
//!
//! - [`TradeResult`] - Outcome of executing a multi-leg trade
//! - [`Fill`] - A successfully executed leg
//! - [`Failure`] - A failed leg with error info

use super::TokenId;

/// A successfully executed leg in a trade.
#[derive(Debug, Clone)]
pub struct Fill {
    /// Token ID for this leg.
    pub token_id: TokenId,
    /// Order ID returned by exchange.
    pub order_id: String,
}

impl Fill {
    /// Create a new fill.
    pub fn new(token_id: TokenId, order_id: impl Into<String>) -> Self {
        Self {
            token_id,
            order_id: order_id.into(),
        }
    }
}

/// A failed leg in a trade.
#[derive(Debug, Clone)]
pub struct Failure {
    /// Token ID for this leg.
    pub token_id: TokenId,
    /// Error message.
    pub error: String,
}

impl Failure {
    /// Create a new failure.
    pub fn new(token_id: TokenId, error: impl Into<String>) -> Self {
        Self {
            token_id,
            error: error.into(),
        }
    }
}

/// Result of executing a multi-leg trade.
#[derive(Debug, Clone)]
pub enum TradeResult {
    /// All legs executed successfully.
    Success {
        /// The successfully filled legs.
        fills: Vec<Fill>,
    },
    /// Some legs executed, some failed.
    Partial {
        /// The legs that were successfully filled.
        fills: Vec<Fill>,
        /// The legs that failed to execute.
        failures: Vec<Failure>,
    },
    /// All legs failed.
    Failed {
        /// The failure reason.
        reason: String,
    },
}

impl TradeResult {
    /// Check if all legs were successful.
    #[must_use]
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// Check if there was a partial fill.
    #[must_use]
    pub const fn is_partial(&self) -> bool {
        matches!(self, Self::Partial { .. })
    }

    /// Check if all legs failed.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// Get fills if any.
    #[must_use]
    pub fn fills(&self) -> &[Fill] {
        match self {
            Self::Success { fills } => fills,
            Self::Partial { fills, .. } => fills,
            Self::Failed { .. } => &[],
        }
    }

    /// Get failures if any.
    #[must_use]
    pub fn failures(&self) -> &[Failure] {
        match self {
            Self::Success { .. } => &[],
            Self::Partial { failures, .. } => failures,
            Self::Failed { .. } => &[],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_new() {
        let fill = Fill::new(TokenId::from("token-1"), "order-1");
        assert_eq!(fill.token_id.as_str(), "token-1");
        assert_eq!(fill.order_id, "order-1");
    }

    #[test]
    fn failure_new() {
        let failure = Failure::new(TokenId::from("token-2"), "insufficient funds");
        assert_eq!(failure.token_id.as_str(), "token-2");
        assert_eq!(failure.error, "insufficient funds");
    }

    #[test]
    fn result_success() {
        let result = TradeResult::Success {
            fills: vec![Fill::new(TokenId::from("t1"), "o1")],
        };
        assert!(result.is_success());
        assert!(!result.is_partial());
        assert!(!result.is_failed());
        assert_eq!(result.fills().len(), 1);
        assert!(result.failures().is_empty());
    }

    #[test]
    fn result_partial() {
        let result = TradeResult::Partial {
            fills: vec![Fill::new(TokenId::from("t1"), "o1")],
            failures: vec![Failure::new(TokenId::from("t2"), "error")],
        };
        assert!(!result.is_success());
        assert!(result.is_partial());
        assert!(!result.is_failed());
        assert_eq!(result.fills().len(), 1);
        assert_eq!(result.failures().len(), 1);
    }

    #[test]
    fn result_failed() {
        let result = TradeResult::Failed {
            reason: "all legs failed".to_string(),
        };
        assert!(!result.is_success());
        assert!(!result.is_partial());
        assert!(result.is_failed());
        assert!(result.fills().is_empty());
        assert!(result.failures().is_empty());
    }
}
