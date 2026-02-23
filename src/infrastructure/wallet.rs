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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::config::settings::Config;

    fn minimal_config() -> Config {
        let toml = r#"
            [logging]
            level = "info"
            format = "pretty"
        "#;
        Config::parse_toml(toml).expect("minimal config should parse")
    }

    // -----------------------------------------------------------------------
    // Type Tests
    // -----------------------------------------------------------------------

    #[test]
    fn wallet_approval_status_fields() {
        let status = WalletApprovalStatus {
            exchange: "TestExchange".to_string(),
            wallet_address: "0x1234".to_string(),
            token: "USDC".to_string(),
            allowance: Decimal::new(1000, 2), // 10.00
            spender: "0x5678".to_string(),
            needs_approval: false,
        };

        assert_eq!(status.exchange, "TestExchange");
        assert_eq!(status.wallet_address, "0x1234");
        assert_eq!(status.token, "USDC");
        assert_eq!(status.allowance, Decimal::new(1000, 2));
        assert_eq!(status.spender, "0x5678");
        assert!(!status.needs_approval);
    }

    #[test]
    fn approval_outcome_approved_variant() {
        let outcome = ApprovalOutcome::Approved {
            tx_hash: "0xabc".to_string(),
            amount: Decimal::new(5000, 2), // 50.00
        };

        match outcome {
            ApprovalOutcome::Approved { tx_hash, amount } => {
                assert_eq!(tx_hash, "0xabc");
                assert_eq!(amount, Decimal::new(5000, 2));
            }
            _ => panic!("Expected Approved variant"),
        }
    }

    #[test]
    fn approval_outcome_already_approved_variant() {
        let outcome = ApprovalOutcome::AlreadyApproved {
            current_allowance: Decimal::new(10000, 2), // 100.00
        };

        match outcome {
            ApprovalOutcome::AlreadyApproved { current_allowance } => {
                assert_eq!(current_allowance, Decimal::new(10000, 2));
            }
            _ => panic!("Expected AlreadyApproved variant"),
        }
    }

    #[test]
    fn approval_outcome_failed_variant() {
        let outcome = ApprovalOutcome::Failed {
            reason: "Insufficient gas".to_string(),
        };

        match outcome {
            ApprovalOutcome::Failed { reason } => {
                assert_eq!(reason, "Insufficient gas");
            }
            _ => panic!("Expected Failed variant"),
        }
    }

    #[test]
    fn sweep_outcome_no_balance_variant() {
        let outcome = SweepOutcome::NoBalance {
            balance: Decimal::ZERO,
        };

        match outcome {
            SweepOutcome::NoBalance { balance } => {
                assert_eq!(balance, Decimal::ZERO);
            }
            _ => panic!("Expected NoBalance variant"),
        }
    }

    #[test]
    fn sweep_outcome_transferred_variant() {
        let outcome = SweepOutcome::Transferred {
            tx_hash: "0xdef".to_string(),
            amount: Decimal::new(25000, 2), // 250.00
        };

        match outcome {
            SweepOutcome::Transferred { tx_hash, amount } => {
                assert_eq!(tx_hash, "0xdef");
                assert_eq!(amount, Decimal::new(25000, 2));
            }
            _ => panic!("Expected Transferred variant"),
        }
    }

    // -----------------------------------------------------------------------
    // Error Path Tests (without polymarket feature)
    // -----------------------------------------------------------------------

    #[cfg(not(feature = "polymarket"))]
    mod without_polymarket {
        use super::*;

        #[tokio::test]
        async fn wallet_address_errors_without_polymarket() {
            let config = minimal_config();
            let result = WalletService::wallet_address(&config);

            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.to_string().contains("polymarket"));
        }

        #[tokio::test]
        async fn get_approval_status_errors_without_polymarket() {
            let config = minimal_config();
            let result = WalletService::get_approval_status(&config).await;

            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.to_string().contains("polymarket"));
        }

        #[tokio::test]
        async fn approve_errors_without_polymarket() {
            let config = minimal_config();
            let result = WalletService::approve(&config, Decimal::new(100, 0)).await;

            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.to_string().contains("polymarket"));
        }

        #[tokio::test]
        async fn usdc_balance_errors_without_polymarket() {
            let config = minimal_config();
            let result = WalletService::usdc_balance(&config).await;

            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.to_string().contains("polymarket"));
        }

        #[tokio::test]
        async fn sweep_usdc_errors_without_polymarket() {
            let config = minimal_config();
            let result =
                WalletService::sweep_usdc(&config, "0x1234567890123456789012345678901234567890")
                    .await;

            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.to_string().contains("polymarket"));
        }
    }

    // -----------------------------------------------------------------------
    // Configuration Validation Tests (with polymarket feature)
    // -----------------------------------------------------------------------

    #[cfg(feature = "polymarket")]
    mod with_polymarket {
        use super::*;
        use crate::error::ConfigError;

        #[test]
        fn polymarket_runtime_config_requires_private_key() {
            let config = minimal_config();
            // Config has no private key set

            let result = WalletService::polymarket_runtime_config(&config);

            assert!(result.is_err());
            match result.unwrap_err() {
                crate::error::Error::Config(ConfigError::MissingField { field }) => {
                    assert_eq!(field, "WALLET_PRIVATE_KEY");
                }
                e => panic!("Expected MissingField error, got: {e:?}"),
            }
        }

        #[test]
        fn polymarket_runtime_config_builds_with_valid_key() {
            let mut config = minimal_config();
            // Set a valid-format private key (64 hex chars)
            config.wallet.private_key = Some(
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            );

            let result = WalletService::polymarket_runtime_config(&config);
            assert!(result.is_ok());

            let runtime = result.unwrap();
            assert!(!runtime.private_key.is_empty());
        }

        #[tokio::test]
        async fn sweep_usdc_validates_address_format() {
            let mut config = minimal_config();
            config.wallet.private_key = Some(
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            );

            // Invalid address format
            let result = WalletService::sweep_usdc(&config, "invalid-address").await;

            assert!(result.is_err());
            let err = result.unwrap_err();
            // Should fail on address parsing, not on the sweep itself
            assert!(err.to_string().contains("to"));
        }
    }

    // -----------------------------------------------------------------------
    // Clone/Debug Tests
    // -----------------------------------------------------------------------

    #[test]
    fn wallet_approval_status_is_clone() {
        let status = WalletApprovalStatus {
            exchange: "Test".to_string(),
            wallet_address: "0x123".to_string(),
            token: "USDC".to_string(),
            allowance: Decimal::ONE,
            spender: "0x456".to_string(),
            needs_approval: true,
        };

        let cloned = status.clone();
        assert_eq!(cloned.exchange, status.exchange);
        assert_eq!(cloned.needs_approval, status.needs_approval);
    }

    #[test]
    fn approval_outcome_is_clone() {
        let outcome = ApprovalOutcome::Approved {
            tx_hash: "0xabc".to_string(),
            amount: Decimal::TEN,
        };

        let cloned = outcome.clone();
        match cloned {
            ApprovalOutcome::Approved { tx_hash, amount } => {
                assert_eq!(tx_hash, "0xabc");
                assert_eq!(amount, Decimal::TEN);
            }
            _ => panic!("Clone should preserve variant"),
        }
    }

    #[test]
    fn sweep_outcome_is_clone() {
        let outcome = SweepOutcome::Transferred {
            tx_hash: "0xdef".to_string(),
            amount: Decimal::new(100, 0),
        };

        let cloned = outcome.clone();
        match cloned {
            SweepOutcome::Transferred { tx_hash, amount } => {
                assert_eq!(tx_hash, "0xdef");
                assert_eq!(amount, Decimal::new(100, 0));
            }
            _ => panic!("Clone should preserve variant"),
        }
    }

    #[test]
    fn types_implement_debug() {
        let status = WalletApprovalStatus {
            exchange: "Test".to_string(),
            wallet_address: "0x123".to_string(),
            token: "USDC".to_string(),
            allowance: Decimal::ONE,
            spender: "0x456".to_string(),
            needs_approval: false,
        };
        let _ = format!("{status:?}");

        let outcome = ApprovalOutcome::Failed {
            reason: "test".to_string(),
        };
        let _ = format!("{outcome:?}");

        let sweep = SweepOutcome::NoBalance {
            balance: Decimal::ZERO,
        };
        let _ = format!("{sweep:?}");
    }
}
