//! Wallet projections for operator-facing adapters.

use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::error::Result;

/// Current approval status for wallet/operator views.
#[derive(Debug, Clone)]
pub struct WalletApprovalStatus {
    pub exchange: String,
    pub wallet_address: String,
    pub token: String,
    pub allowance: Decimal,
    pub spender: String,
    pub needs_approval: bool,
}

/// Outcome of an approval operation.
#[derive(Debug, Clone)]
pub enum ApprovalOutcome {
    Approved { tx_hash: String, amount: Decimal },
    AlreadyApproved { current_allowance: Decimal },
    Failed { reason: String },
}

/// Outcome of a sweep operation.
#[derive(Debug, Clone)]
pub enum SweepOutcome {
    NoBalance { balance: Decimal },
    Transferred { tx_hash: String, amount: Decimal },
}

/// Wallet use-cases for operator-facing adapters.
#[async_trait]
pub trait WalletOperator: Send + Sync {
    /// Get wallet approval status for configured exchange.
    async fn wallet_status(&self, config_toml: &str) -> Result<WalletApprovalStatus>;

    /// Submit approval transaction.
    async fn wallet_approve(&self, config_toml: &str, amount: Decimal) -> Result<ApprovalOutcome>;

    /// Resolve configured wallet address.
    fn wallet_address(&self, config_toml: &str) -> Result<String>;

    /// Load spendable USDC balance.
    async fn wallet_balance(&self, config_toml: &str) -> Result<Decimal>;

    /// Sweep balance to destination after exchange/network validation.
    async fn wallet_sweep(
        &self,
        config_toml: &str,
        to: &str,
        asset: &str,
        network: &str,
    ) -> Result<SweepOutcome>;
}
