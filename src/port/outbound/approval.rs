//! Token approval port for ERC-20 spending workflows.
//!
//! Defines traits and types for managing token spending approvals required
//! by exchanges that interact with smart contracts.

use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::error::Result;

/// Result of a token approval operation.
#[derive(Debug, Clone)]
pub enum ApprovalResult {
    /// Approval transaction was submitted successfully.
    Approved {
        /// Transaction hash for tracking.
        tx_hash: String,

        /// Amount approved for spending.
        amount: Decimal,
    },

    /// Sufficient allowance already exists.
    AlreadyApproved {
        /// Current spending allowance.
        current_allowance: Decimal,
    },

    /// Approval transaction failed.
    Failed {
        /// Human-readable error description.
        reason: String,
    },
}

/// Current token approval status.
#[derive(Debug, Clone)]
pub struct ApprovalStatus {
    /// Token symbol (e.g., "USDC").
    pub token: String,

    /// Current spending allowance.
    pub allowance: Decimal,

    /// Spender contract address.
    pub spender: String,

    /// Whether additional approval is needed for trading.
    pub needs_approval: bool,
}

/// Port for managing ERC-20 token approvals.
///
/// Implementations handle the approval workflow for exchanges that require
/// token spending permissions (e.g., DEX contracts).
///
/// # Thread Safety
///
/// Implementations must be thread-safe (`Send + Sync`).
///
/// # Errors
///
/// Methods return [`Result`] for blockchain interaction failures.
#[async_trait]
pub trait TokenApproval: Send + Sync {
    /// Retrieve the current approval status.
    ///
    /// # Errors
    ///
    /// Returns an error if the allowance cannot be read from the blockchain.
    async fn get_approval_status(&self) -> Result<ApprovalStatus>;

    /// Submit a token approval transaction.
    ///
    /// # Arguments
    ///
    /// * `amount` - Amount to approve for spending.
    ///
    /// # Errors
    ///
    /// Returns an error if the transaction cannot be submitted or confirmed.
    async fn approve(&self, amount: Decimal) -> Result<ApprovalResult>;

    /// Return the exchange name for logging and display.
    fn exchange_name(&self) -> &'static str;

    /// Return the token symbol being approved.
    fn token_name(&self) -> &'static str;
}
