//! Wallet operations facade for CLI.
//!
//! Provides CLI-friendly types and methods for wallet-related operations
//! like token approvals, balance queries, and fund transfers. This module
//! abstracts exchange-specific wallet implementations behind a unified API.

use rust_decimal::Decimal;

use crate::error::ConfigError;
use crate::error::Result;
use crate::infrastructure::config::settings::{Config, Exchange};

#[cfg(feature = "polymarket")]
use crate::adapter::outbound::polymarket::approval::{
    PolymarketApproval, SweepResult as PolymarketSweepResult,
};
#[cfg(feature = "polymarket")]
use crate::adapter::outbound::polymarket::settings::PolymarketRuntimeConfig;
#[cfg(feature = "polymarket")]
use crate::port::{outbound::approval::ApprovalResult, outbound::approval::TokenApproval};

/// Current token approval status for CLI display.
///
/// Provides a CLI-friendly representation of approval state without
/// exposing core layer types.
#[derive(Debug, Clone)]
pub struct WalletApprovalStatus {
    /// Name of the exchange.
    pub exchange: String,
    /// Wallet address that owns the tokens.
    pub wallet_address: String,
    /// Token symbol (e.g., "USDC").
    pub token: String,
    /// Current allowance amount in token units.
    pub allowance: Decimal,
    /// Contract address approved to spend tokens.
    pub spender: String,
    /// Whether additional approval is needed for trading.
    pub needs_approval: bool,
}

/// Outcome of a token approval operation.
///
/// Provides a CLI-friendly result type without exposing core layer types.
#[derive(Debug, Clone)]
pub enum ApprovalOutcome {
    /// Approval transaction was submitted and confirmed.
    Approved {
        /// Transaction hash on the blockchain.
        tx_hash: String,
        /// Amount approved in token units.
        amount: Decimal,
    },
    /// Token was already approved for the requested amount or more.
    AlreadyApproved {
        /// Current allowance in token units.
        current_allowance: Decimal,
    },
    /// Approval transaction failed.
    Failed {
        /// Human-readable error description.
        reason: String,
    },
}

/// Outcome of a balance sweep operation.
#[derive(Debug, Clone)]
pub enum SweepOutcome {
    /// No balance available to sweep.
    NoBalance {
        /// Current balance (should be zero or near-zero).
        balance: Decimal,
    },
    /// Sweep transaction was submitted and confirmed.
    Transferred {
        /// Transaction hash on the blockchain.
        tx_hash: String,
        /// Amount transferred in token units.
        amount: Decimal,
    },
}

/// Wallet service providing CLI operations.
///
/// Dispatches to the appropriate exchange-specific implementation
/// based on configuration. All methods are async and require a valid
/// wallet configuration.
pub struct WalletService;

impl WalletService {
    /// Build Polymarket runtime configuration from app config.
    #[cfg(feature = "polymarket")]
    fn polymarket_runtime_config(config: &Config) -> Result<PolymarketRuntimeConfig> {
        let private_key =
            config
                .wallet
                .private_key
                .as_ref()
                .cloned()
                .ok_or(ConfigError::MissingField {
                    field: "WALLET_PRIVATE_KEY",
                })?;
        let network = config.network();
        Ok(PolymarketRuntimeConfig {
            private_key,
            chain_id: network.chain_id,
            api_url: network.api_url,
            environment: network.environment,
        })
    }

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

    /// Get the wallet address for the configured exchange.
    ///
    /// # Errors
    ///
    /// Returns an error if the wallet is not configured.
    pub fn wallet_address(config: &Config) -> Result<String> {
        match config.exchange {
            Exchange::Polymarket => Self::polymarket_wallet_address(config),
        }
    }

    /// Get the USDC balance for the configured exchange.
    ///
    /// # Errors
    ///
    /// Returns an error if the wallet is not configured or the balance
    /// query fails.
    pub async fn usdc_balance(config: &Config) -> Result<Decimal> {
        match config.exchange {
            Exchange::Polymarket => Self::polymarket_usdc_balance(config).await,
        }
    }

    /// Sweep the full USDC balance to the specified address.
    ///
    /// Transfers all available USDC to the destination address.
    ///
    /// # Errors
    ///
    /// Returns an error if the wallet is not configured, the destination
    /// address is invalid, or the transfer fails.
    pub async fn sweep_usdc(config: &Config, to: &str) -> Result<SweepOutcome> {
        match config.exchange {
            Exchange::Polymarket => Self::sweep_polymarket(config, to).await,
        }
    }

    // --- Polymarket implementation ---

    #[cfg(feature = "polymarket")]
    async fn get_polymarket_status(config: &Config) -> Result<WalletApprovalStatus> {
        let runtime = Self::polymarket_runtime_config(config)?;
        let approval = PolymarketApproval::new(&runtime)?;
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
        let runtime = Self::polymarket_runtime_config(config)?;
        let approval = PolymarketApproval::new(&runtime)?;
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

    #[cfg(feature = "polymarket")]
    fn polymarket_wallet_address(config: &Config) -> Result<String> {
        let runtime = Self::polymarket_runtime_config(config)?;
        let approval = PolymarketApproval::new(&runtime)?;
        Ok(approval.wallet_address().to_string())
    }

    #[cfg(not(feature = "polymarket"))]
    fn polymarket_wallet_address(_config: &Config) -> Result<String> {
        Err(crate::error::ConfigError::InvalidValue {
            field: "exchange",
            reason: "Polymarket support requires the 'polymarket' feature".to_string(),
        }
        .into())
    }

    #[cfg(feature = "polymarket")]
    async fn polymarket_usdc_balance(config: &Config) -> Result<Decimal> {
        let runtime = Self::polymarket_runtime_config(config)?;
        let approval = PolymarketApproval::new(&runtime)?;
        approval.usdc_balance().await
    }

    #[cfg(not(feature = "polymarket"))]
    async fn polymarket_usdc_balance(_config: &Config) -> Result<Decimal> {
        Err(crate::error::ConfigError::InvalidValue {
            field: "exchange",
            reason: "Polymarket support requires the 'polymarket' feature".to_string(),
        }
        .into())
    }

    #[cfg(feature = "polymarket")]
    async fn sweep_polymarket(config: &Config, to: &str) -> Result<SweepOutcome> {
        use alloy_primitives::Address;
        use std::str::FromStr;

        let to_address = Address::from_str(to).map_err(|e| ConfigError::InvalidValue {
            field: "to",
            reason: e.to_string(),
        })?;

        let runtime = Self::polymarket_runtime_config(config)?;
        let approval = PolymarketApproval::new(&runtime)?;
        let result = approval.sweep_usdc(to_address).await?;

        Ok(match result {
            PolymarketSweepResult::NoBalance { balance } => SweepOutcome::NoBalance { balance },
            PolymarketSweepResult::Transferred { tx_hash, amount } => {
                SweepOutcome::Transferred { tx_hash, amount }
            }
        })
    }

    #[cfg(not(feature = "polymarket"))]
    async fn sweep_polymarket(_config: &Config, _to: &str) -> Result<SweepOutcome> {
        Err(crate::error::ConfigError::InvalidValue {
            field: "exchange",
            reason: "Polymarket support requires the 'polymarket' feature".to_string(),
        }
        .into())
    }
}
