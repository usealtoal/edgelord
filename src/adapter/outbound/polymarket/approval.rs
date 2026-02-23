//! Polymarket USDC token approval.
//!
//! Handles ERC-20 approval for USDC spending on Polymarket's CTF Exchange
//! contract. Supports both Polygon mainnet and Amoy testnet environments.

use std::str::FromStr;

use alloy_primitives::{Address, U256};
use alloy_provider::ProviderBuilder;
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_types::sol;
use async_trait::async_trait;
use polymarket_client_sdk::auth::Signer as _;
use rust_decimal::Decimal;
use tracing::info;

use super::settings::{Environment, PolymarketRuntimeConfig};
use crate::error::{ConfigError, ExecutionError, Result};
use crate::port::{
    outbound::approval::ApprovalResult, outbound::approval::ApprovalStatus,
    outbound::approval::TokenApproval,
};

/// Native USDC contract address on Polygon mainnet.
const USDC_NATIVE_MAINNET: &str = "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359";

/// USDC contract address on Amoy testnet.
const USDC_TESTNET: &str = "0x2E8D98fd126a32362F2Bd8aA427E59a1ec63F780";

/// Polymarket CTF Exchange contract address on mainnet.
const CTF_EXCHANGE_MAINNET: &str = "0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E";

/// Polymarket CTF Exchange contract address on testnet.
const CTF_EXCHANGE_TESTNET: &str = "0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E";

/// Public Polygon mainnet RPC endpoint.
const POLYGON_RPC: &str = "https://polygon-rpc.com";

/// Public Amoy testnet RPC endpoint.
const AMOY_RPC: &str = "https://rpc-amoy.polygon.technology";

/// Number of decimals for USDC token.
const USDC_DECIMALS: u32 = 6;

// ERC-20 interface (minimal for approval)
sol! {
    #[sol(rpc)]
    contract IERC20 {
        function allowance(address owner, address spender) external view returns (uint256);
        function approve(address spender, uint256 amount) external returns (bool);
        function balanceOf(address account) external view returns (uint256);
        function transfer(address to, uint256 amount) external returns (bool);
    }
}

/// Result of a USDC sweep operation.
#[derive(Debug, Clone)]
pub enum SweepResult {
    /// No balance available to sweep.
    NoBalance {
        /// Current balance (should be zero or negligible).
        balance: Decimal,
    },
    /// Sweep transfer completed successfully.
    Transferred {
        /// Transaction hash of the transfer.
        tx_hash: String,
        /// Amount transferred in USD.
        amount: Decimal,
    },
}

/// Token approval handler for Polymarket.
///
/// Manages ERC-20 token approvals and balance queries for the trading wallet.
/// Implements the [`TokenApproval`] trait for integration with the approval
/// workflow.
pub struct PolymarketApproval {
    /// Local signer derived from the wallet private key.
    signer: PrivateKeySigner,
    /// Current deployment environment (testnet or mainnet).
    environment: Environment,
}

impl PolymarketApproval {
    /// Create a new approval handler from runtime configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the private key is missing or invalid.
    pub fn new(config: &PolymarketRuntimeConfig) -> Result<Self> {
        if config.private_key.trim().is_empty() {
            return Err(ConfigError::MissingField {
                field: "WALLET_PRIVATE_KEY",
            }
            .into());
        }

        let signer = PrivateKeySigner::from_str(&config.private_key)
            .map_err(|e| ConfigError::InvalidValue {
                field: "WALLET_PRIVATE_KEY",
                reason: e.to_string(),
            })?
            .with_chain_id(Some(config.chain_id));

        Ok(Self {
            signer,
            environment: config.environment,
        })
    }

    /// Return the RPC URL for the current environment.
    fn rpc_url(&self) -> &'static str {
        match self.environment {
            Environment::Mainnet => POLYGON_RPC,
            Environment::Testnet => AMOY_RPC,
        }
    }

    /// Return the USDC contract address for the current environment.
    fn usdc_address(&self) -> Result<Address> {
        let addr = match self.environment {
            Environment::Mainnet => USDC_NATIVE_MAINNET,
            Environment::Testnet => USDC_TESTNET,
        };
        Address::from_str(addr).map_err(|e| {
            ConfigError::InvalidValue {
                field: "usdc_address",
                reason: e.to_string(),
            }
            .into()
        })
    }

    /// Return the CTF Exchange (spender) contract address.
    fn spender_address(&self) -> Result<Address> {
        let addr = match self.environment {
            Environment::Mainnet => CTF_EXCHANGE_MAINNET,
            Environment::Testnet => CTF_EXCHANGE_TESTNET,
        };
        Address::from_str(addr).map_err(|e| {
            ConfigError::InvalidValue {
                field: "spender_address",
                reason: e.to_string(),
            }
            .into()
        })
    }

    /// Convert decimal dollars to USDC base units (6 decimals).
    fn to_usdc_units(amount: Decimal) -> U256 {
        let scaled = amount * Decimal::from(10u64.pow(USDC_DECIMALS));
        let int_amount = scaled.trunc().to_string().parse::<u128>().unwrap_or(0);
        U256::from(int_amount)
    }

    /// Convert USDC base units to decimal dollars.
    fn from_usdc_units(units: U256) -> Decimal {
        let int_val: u128 = units.try_into().unwrap_or(u128::MAX);
        Decimal::from(int_val) / Decimal::from(10u64.pow(USDC_DECIMALS))
    }

    /// Return the wallet address derived from the private key.
    #[must_use]
    pub fn wallet_address(&self) -> Address {
        self.signer.address()
    }

    /// Query the USDC balance for the wallet.
    ///
    /// # Errors
    ///
    /// Returns an error if the RPC call fails.
    pub async fn usdc_balance(&self) -> Result<Decimal> {
        let rpc_url: url::Url =
            self.rpc_url()
                .parse()
                .map_err(|e: url::ParseError| ConfigError::InvalidValue {
                    field: "rpc_url",
                    reason: e.to_string(),
                })?;
        let provider = ProviderBuilder::new().connect_http(rpc_url);

        let usdc = IERC20::new(self.usdc_address()?, &provider);
        let owner = self.signer.address();
        let balance: U256 =
            usdc.balanceOf(owner).call().await.map_err(|e| {
                ExecutionError::SubmissionFailed(format!("Failed to get balance: {e}"))
            })?;

        Ok(Self::from_usdc_units(balance))
    }

    /// Transfer the full USDC balance to another address.
    ///
    /// # Errors
    ///
    /// Returns an error if the balance query or transfer transaction fails.
    pub async fn sweep_usdc(&self, to: Address) -> Result<SweepResult> {
        let balance = self.usdc_balance().await?;
        if balance <= Decimal::ZERO {
            return Ok(SweepResult::NoBalance { balance });
        }

        let wallet = alloy_provider::network::EthereumWallet::from(self.signer.clone());
        let rpc_url: url::Url =
            self.rpc_url()
                .parse()
                .map_err(|e: url::ParseError| ConfigError::InvalidValue {
                    field: "rpc_url",
                    reason: e.to_string(),
                })?;
        let provider = ProviderBuilder::new().wallet(wallet).connect_http(rpc_url);

        let usdc = IERC20::new(self.usdc_address()?, &provider);
        let amount_units = Self::to_usdc_units(balance);

        let pending_tx = usdc.transfer(to, amount_units).send().await.map_err(|e| {
            ExecutionError::SubmissionFailed(format!("Failed to send transfer: {e}"))
        })?;

        let receipt = pending_tx
            .get_receipt()
            .await
            .map_err(|e| ExecutionError::SubmissionFailed(format!("Failed to get receipt: {e}")))?;

        let tx_hash = format!("{:?}", receipt.transaction_hash);

        Ok(SweepResult::Transferred {
            tx_hash,
            amount: balance,
        })
    }
}

#[async_trait]
impl TokenApproval for PolymarketApproval {
    async fn get_approval_status(&self) -> Result<ApprovalStatus> {
        let rpc_url: url::Url =
            self.rpc_url()
                .parse()
                .map_err(|e: url::ParseError| ConfigError::InvalidValue {
                    field: "rpc_url",
                    reason: e.to_string(),
                })?;
        let provider = ProviderBuilder::new().connect_http(rpc_url);

        let usdc = IERC20::new(self.usdc_address()?, &provider);
        let owner = self.signer.address();
        let spender = self.spender_address()?;

        // Get current allowance
        let allowance: U256 = usdc.allowance(owner, spender).call().await.map_err(|e| {
            ExecutionError::SubmissionFailed(format!("Failed to get allowance: {e}"))
        })?;

        let allowance_decimal = Self::from_usdc_units(allowance);

        Ok(ApprovalStatus {
            token: "USDC".to_string(),
            allowance: allowance_decimal,
            spender: format!("{spender}"),
            needs_approval: allowance_decimal < Decimal::from(1), // Needs approval if < $1
        })
    }

    async fn approve(&self, amount: Decimal) -> Result<ApprovalResult> {
        info!(
            address = %self.signer.address(),
            amount = %amount,
            environment = %self.environment,
            "Approving USDC spending"
        );

        // Check current allowance first
        let status = self.get_approval_status().await?;
        if status.allowance >= amount {
            return Ok(ApprovalResult::AlreadyApproved {
                current_allowance: status.allowance,
            });
        }

        // Build provider with signer for transactions
        let wallet = alloy_provider::network::EthereumWallet::from(self.signer.clone());
        let rpc_url: url::Url =
            self.rpc_url()
                .parse()
                .map_err(|e: url::ParseError| ConfigError::InvalidValue {
                    field: "rpc_url",
                    reason: e.to_string(),
                })?;
        let provider = ProviderBuilder::new().wallet(wallet).connect_http(rpc_url);

        let usdc = IERC20::new(self.usdc_address()?, &provider);
        let spender = self.spender_address()?;
        let amount_units = Self::to_usdc_units(amount);

        // Submit approval transaction
        let pending_tx = usdc
            .approve(spender, amount_units)
            .send()
            .await
            .map_err(|e| {
                ExecutionError::SubmissionFailed(format!("Failed to send approval: {e}"))
            })?;

        let receipt = pending_tx
            .get_receipt()
            .await
            .map_err(|e| ExecutionError::SubmissionFailed(format!("Failed to get receipt: {e}")))?;

        let tx_hash = format!("{:?}", receipt.transaction_hash);

        info!(tx_hash = %tx_hash, "Approval transaction confirmed");

        Ok(ApprovalResult::Approved { tx_hash, amount })
    }

    fn exchange_name(&self) -> &'static str {
        "Polymarket"
    }

    fn token_name(&self) -> &'static str {
        "USDC"
    }
}
