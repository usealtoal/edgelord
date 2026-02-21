//! Token approval traits and implementations.
//!
//! Provides a generic interface for approving token spending on different exchanges.

use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::error::Result;

/// Result of a token approval operation.
#[derive(Debug, Clone)]
pub enum ApprovalResult {
    /// Approval transaction submitted successfully.
    Approved {
        /// Transaction hash.
        tx_hash: String,
        /// Amount approved.
        amount: Decimal,
    },
    /// Already approved for sufficient amount.
    AlreadyApproved {
        /// Current allowance.
        current_allowance: Decimal,
    },
    /// Approval failed.
    Failed {
        /// Error message.
        reason: String,
    },
}

/// Current approval status for a token.
#[derive(Debug, Clone)]
pub struct ApprovalStatus {
    /// Token symbol (e.g., "USDC").
    pub token: String,
    /// Current allowance.
    pub allowance: Decimal,
    /// Spender address (the exchange contract).
    pub spender: String,
    /// Whether approval is needed for the requested amount.
    pub needs_approval: bool,
}

/// Trait for exchanges that require token approvals.
#[async_trait]
pub trait TokenApproval: Send + Sync {
    /// Get the current approval status.
    async fn get_approval_status(&self) -> Result<ApprovalStatus>;

    /// Approve token spending for the exchange.
    ///
    /// # Arguments
    ///
    /// * `amount` - Amount to approve in token units (e.g., dollars for USDC).
    async fn approve(&self, amount: Decimal) -> Result<ApprovalResult>;

    /// Get the exchange name for display purposes.
    fn exchange_name(&self) -> &'static str;

    /// Get the token name being approved.
    fn token_name(&self) -> &'static str;
}
