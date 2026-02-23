//! Port for token approval workflows.

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

/// Port for exchanges that require token approvals.
#[async_trait]
pub trait TokenApproval: Send + Sync {
    /// Get the current approval status.
    async fn get_approval_status(&self) -> Result<ApprovalStatus>;

    /// Approve token spending for the exchange.
    async fn approve(&self, amount: Decimal) -> Result<ApprovalResult>;

    /// Exchange name for display and logging.
    fn exchange_name(&self) -> &'static str;

    /// Token name being approved.
    fn token_name(&self) -> &'static str;
}
