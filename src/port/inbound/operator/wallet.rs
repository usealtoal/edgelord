//! Wallet management projection types for operator-facing adapters.
//!
//! Defines view models for wallet status, token approvals, and balance
//! management through operator interfaces.

use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::error::Result;

/// Current token approval status for display.
#[derive(Debug, Clone)]
pub struct WalletApprovalStatus {
    /// Exchange name.
    pub exchange: String,

    /// Wallet address.
    pub wallet_address: String,

    /// Token symbol (e.g., "USDC").
    pub token: String,

    /// Current spending allowance.
    pub allowance: Decimal,

    /// Spender contract address.
    pub spender: String,

    /// Whether additional approval is needed.
    pub needs_approval: bool,
}

/// Outcome of a token approval operation.
#[derive(Debug, Clone)]
pub enum ApprovalOutcome {
    /// Approval transaction was submitted successfully.
    Approved {
        /// Transaction hash.
        tx_hash: String,

        /// Approved amount.
        amount: Decimal,
    },

    /// Sufficient approval already exists.
    AlreadyApproved {
        /// Current allowance amount.
        current_allowance: Decimal,
    },

    /// Approval operation failed.
    Failed {
        /// Failure reason.
        reason: String,
    },
}

/// Outcome of a balance sweep operation.
#[derive(Debug, Clone)]
pub enum SweepOutcome {
    /// No balance available to sweep.
    NoBalance {
        /// Current balance (zero or dust).
        balance: Decimal,
    },

    /// Balance was transferred successfully.
    Transferred {
        /// Transaction hash.
        tx_hash: String,

        /// Amount transferred.
        amount: Decimal,
    },
}

/// Wallet management use-cases for operator-facing adapters.
///
/// # Thread Safety
///
/// Implementations must be thread-safe (`Send + Sync`).
#[async_trait]
pub trait WalletOperator: Send + Sync {
    /// Retrieve the current wallet approval status.
    ///
    /// # Arguments
    ///
    /// * `config_toml` - Raw TOML configuration content.
    ///
    /// # Errors
    ///
    /// Returns an error if the wallet is not configured or status cannot be retrieved.
    async fn wallet_status(&self, config_toml: &str) -> Result<WalletApprovalStatus>;

    /// Submit a token approval transaction.
    ///
    /// # Arguments
    ///
    /// * `config_toml` - Raw TOML configuration content.
    /// * `amount` - Amount to approve for spending.
    ///
    /// # Errors
    ///
    /// Returns an error if the transaction cannot be submitted.
    async fn wallet_approve(&self, config_toml: &str, amount: Decimal) -> Result<ApprovalOutcome>;

    /// Retrieve the configured wallet address.
    ///
    /// # Arguments
    ///
    /// * `config_toml` - Raw TOML configuration content.
    ///
    /// # Errors
    ///
    /// Returns an error if no wallet is configured.
    fn wallet_address(&self, config_toml: &str) -> Result<String>;

    /// Retrieve the current spendable balance.
    ///
    /// # Arguments
    ///
    /// * `config_toml` - Raw TOML configuration content.
    ///
    /// # Errors
    ///
    /// Returns an error if the balance cannot be retrieved.
    async fn wallet_balance(&self, config_toml: &str) -> Result<Decimal>;

    /// Sweep wallet balance to a destination address.
    ///
    /// # Arguments
    ///
    /// * `config_toml` - Raw TOML configuration content.
    /// * `to` - Destination wallet address.
    /// * `asset` - Asset to sweep (e.g., "USDC").
    /// * `network` - Network for the transfer (e.g., "polygon").
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails or the transfer cannot be submitted.
    async fn wallet_sweep(
        &self,
        config_toml: &str,
        to: &str,
        asset: &str,
        network: &str,
    ) -> Result<SweepOutcome>;
}
