//! Wallet operations facade for CLI.
//!
//! Provides CLI-friendly types and methods for wallet-related operations
//! like token approvals without exposing core layer internals.

use rust_decimal::Decimal;

use crate::app::{Config, Exchange};
use crate::error::Result;

#[cfg(feature = "polymarket")]
use crate::core::exchange::{polymarket::PolymarketApproval, ApprovalResult, TokenApproval};

/// Current approval status for CLI display.
///
/// This is a CLI-friendly representation that doesn't expose
/// core layer types to the CLI.
#[derive(Debug, Clone)]
pub struct WalletApprovalStatus {
    /// Exchange name.
    pub exchange: String,
    /// Wallet address.
    pub wallet_address: String,
    /// Token symbol (e.g., "USDC").
    pub token: String,
    /// Current allowance amount.
    pub allowance: Decimal,
    /// Spender contract address.
    pub spender: String,
    /// Whether additional approval is needed.
    pub needs_approval: bool,
}

/// Outcome of an approval operation.
///
/// CLI-friendly wrapper that doesn't expose core layer types.
#[derive(Debug, Clone)]
pub enum ApprovalOutcome {
    /// Approval transaction was submitted and confirmed.
    Approved {
        /// Transaction hash.
        tx_hash: String,
        /// Amount approved.
        amount: Decimal,
    },
    /// Token was already approved for the requested amount.
    AlreadyApproved {
        /// Current allowance.
        current_allowance: Decimal,
    },
    /// Approval failed.
    Failed {
        /// Error reason.
        reason: String,
    },
}

/// Wallet service providing CLI operations.
///
/// Dispatches to the appropriate exchange-specific implementation
/// based on the configuration.
pub struct WalletService;

impl WalletService {
    /// Get current approval status for the configured exchange.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The wallet is not configured
    /// - The exchange doesn't support approval operations
    /// - The status check fails
    pub async fn get_approval_status(config: &Config) -> Result<WalletApprovalStatus> {
        match config.exchange {
            Exchange::Polymarket => Self::get_polymarket_status(config).await,
        }
    }

    /// Approve token spending for the configured exchange.
    ///
    /// # Arguments
    ///
    /// * `config` - Application configuration
    /// * `amount` - Amount to approve in token units (e.g., dollars for USDC)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The wallet is not configured
    /// - The exchange doesn't support approval operations
    /// - The approval transaction fails
    pub async fn approve(config: &Config, amount: Decimal) -> Result<ApprovalOutcome> {
        match config.exchange {
            Exchange::Polymarket => Self::approve_polymarket(config, amount).await,
        }
    }

    // --- Polymarket implementation ---

    #[cfg(feature = "polymarket")]
    async fn get_polymarket_status(config: &Config) -> Result<WalletApprovalStatus> {
        let approval = PolymarketApproval::new(config)?;
        let wallet_address = approval.wallet_address().to_string();
        let status = approval.get_approval_status().await?;

        Ok(WalletApprovalStatus {
            exchange: "Polymarket".to_string(),
            wallet_address,
            token: status.token,
            allowance: status.allowance,
            spender: status.spender,
            needs_approval: status.needs_approval,
        })
    }

    #[cfg(not(feature = "polymarket"))]
    async fn get_polymarket_status(_config: &Config) -> Result<WalletApprovalStatus> {
        Err(crate::error::ConfigError::InvalidValue {
            field: "exchange",
            reason: "Polymarket support requires the 'polymarket' feature".to_string(),
        }
        .into())
    }

    #[cfg(feature = "polymarket")]
    async fn approve_polymarket(config: &Config, amount: Decimal) -> Result<ApprovalOutcome> {
        let approval = PolymarketApproval::new(config)?;
        let result = approval.approve(amount).await?;

        Ok(match result {
            ApprovalResult::Approved { tx_hash, amount } => {
                ApprovalOutcome::Approved { tx_hash, amount }
            }
            ApprovalResult::AlreadyApproved { current_allowance } => {
                ApprovalOutcome::AlreadyApproved { current_allowance }
            }
            ApprovalResult::Failed { reason } => ApprovalOutcome::Failed { reason },
        })
    }

    #[cfg(not(feature = "polymarket"))]
    async fn approve_polymarket(_config: &Config, _amount: Decimal) -> Result<ApprovalOutcome> {
        Err(crate::error::ConfigError::InvalidValue {
            field: "exchange",
            reason: "Polymarket support requires the 'polymarket' feature".to_string(),
        }
        .into())
    }
}
