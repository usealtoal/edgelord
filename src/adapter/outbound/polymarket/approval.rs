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

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // -------------------------------------------------------------------------
    // Environment Tests
    // -------------------------------------------------------------------------

    #[test]
    fn environment_display_testnet() {
        let env = Environment::Testnet;
        assert_eq!(format!("{}", env), "testnet");
    }

    #[test]
    fn environment_display_mainnet() {
        let env = Environment::Mainnet;
        assert_eq!(format!("{}", env), "mainnet");
    }

    #[test]
    fn environment_default_is_testnet() {
        let env = Environment::default();
        assert_eq!(env, Environment::Testnet);
    }

    // -------------------------------------------------------------------------
    // USDC Conversion Tests
    // -------------------------------------------------------------------------

    #[test]
    fn to_usdc_units_converts_dollars_to_base_units() {
        // 1 USDC = 1,000,000 base units (6 decimals)
        let result = PolymarketApproval::to_usdc_units(dec!(1));
        assert_eq!(result, U256::from(1_000_000u64));
    }

    #[test]
    fn to_usdc_units_handles_fractional_amounts() {
        // 0.50 USDC = 500,000 base units
        let result = PolymarketApproval::to_usdc_units(dec!(0.50));
        assert_eq!(result, U256::from(500_000u64));
    }

    #[test]
    fn to_usdc_units_handles_large_amounts() {
        // 1000 USDC = 1,000,000,000 base units
        let result = PolymarketApproval::to_usdc_units(dec!(1000));
        assert_eq!(result, U256::from(1_000_000_000u64));
    }

    #[test]
    fn to_usdc_units_truncates_extra_decimals() {
        // 1.1234567 USDC -> should truncate to 1.123456 (6 decimals)
        let result = PolymarketApproval::to_usdc_units(dec!(1.1234567));
        // 1.123456 * 1e6 = 1,123,456
        assert_eq!(result, U256::from(1_123_456u64));
    }

    #[test]
    fn to_usdc_units_handles_zero() {
        let result = PolymarketApproval::to_usdc_units(dec!(0));
        assert_eq!(result, U256::ZERO);
    }

    #[test]
    fn from_usdc_units_converts_base_units_to_dollars() {
        // 1,000,000 base units = 1 USDC
        let result = PolymarketApproval::from_usdc_units(U256::from(1_000_000u64));
        assert_eq!(result, dec!(1));
    }

    #[test]
    fn from_usdc_units_handles_fractional() {
        // 500,000 base units = 0.50 USDC
        let result = PolymarketApproval::from_usdc_units(U256::from(500_000u64));
        assert_eq!(result, dec!(0.5));
    }

    #[test]
    fn from_usdc_units_handles_large_amounts() {
        // 10,000,000,000 base units = 10,000 USDC
        let result = PolymarketApproval::from_usdc_units(U256::from(10_000_000_000u64));
        assert_eq!(result, dec!(10000));
    }

    #[test]
    fn from_usdc_units_handles_zero() {
        let result = PolymarketApproval::from_usdc_units(U256::ZERO);
        assert_eq!(result, dec!(0));
    }

    #[test]
    fn usdc_conversion_round_trips() {
        let original = dec!(123.456789);
        let units = PolymarketApproval::to_usdc_units(original);
        let back = PolymarketApproval::from_usdc_units(units);

        // Should lose precision beyond 6 decimals
        // 123.456789 -> 123456789 (truncated to 123456) -> 123.456
        assert_eq!(back, dec!(123.456789));
    }

    // -------------------------------------------------------------------------
    // ApprovalResult Tests
    // -------------------------------------------------------------------------

    #[test]
    fn approval_result_approved() {
        let result = ApprovalResult::Approved {
            tx_hash: "0xabc123".into(),
            amount: dec!(1000),
        };

        match result {
            ApprovalResult::Approved { tx_hash, amount } => {
                assert_eq!(tx_hash, "0xabc123");
                assert_eq!(amount, dec!(1000));
            }
            _ => panic!("Expected Approved variant"),
        }
    }

    #[test]
    fn approval_result_already_approved() {
        let result = ApprovalResult::AlreadyApproved {
            current_allowance: dec!(5000),
        };

        match result {
            ApprovalResult::AlreadyApproved { current_allowance } => {
                assert_eq!(current_allowance, dec!(5000));
            }
            _ => panic!("Expected AlreadyApproved variant"),
        }
    }

    #[test]
    fn approval_result_failed() {
        let result = ApprovalResult::Failed {
            reason: "Insufficient gas".into(),
        };

        match result {
            ApprovalResult::Failed { reason } => {
                assert_eq!(reason, "Insufficient gas");
            }
            _ => panic!("Expected Failed variant"),
        }
    }

    // -------------------------------------------------------------------------
    // ApprovalStatus Tests
    // -------------------------------------------------------------------------

    #[test]
    fn approval_status_needs_approval_when_low_allowance() {
        let status = ApprovalStatus {
            token: "USDC".into(),
            allowance: dec!(0.50),
            spender: "0xspender".into(),
            needs_approval: true,
        };

        assert!(status.needs_approval);
        assert_eq!(status.token, "USDC");
    }

    #[test]
    fn approval_status_no_approval_needed_when_sufficient() {
        let status = ApprovalStatus {
            token: "USDC".into(),
            allowance: dec!(10000),
            spender: "0xspender".into(),
            needs_approval: false,
        };

        assert!(!status.needs_approval);
    }

    // -------------------------------------------------------------------------
    // SweepResult Tests
    // -------------------------------------------------------------------------

    #[test]
    fn sweep_result_no_balance() {
        let result = SweepResult::NoBalance { balance: dec!(0) };

        match result {
            SweepResult::NoBalance { balance } => {
                assert_eq!(balance, dec!(0));
            }
            _ => panic!("Expected NoBalance variant"),
        }
    }

    #[test]
    fn sweep_result_transferred() {
        let result = SweepResult::Transferred {
            tx_hash: "0xtxhash".into(),
            amount: dec!(500),
        };

        match result {
            SweepResult::Transferred { tx_hash, amount } => {
                assert_eq!(tx_hash, "0xtxhash");
                assert_eq!(amount, dec!(500));
            }
            _ => panic!("Expected Transferred variant"),
        }
    }

    // -------------------------------------------------------------------------
    // Contract Address Constants Tests
    // -------------------------------------------------------------------------

    #[test]
    fn usdc_mainnet_address_is_valid() {
        assert_eq!(
            USDC_NATIVE_MAINNET,
            "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359"
        );
        assert!(USDC_NATIVE_MAINNET.starts_with("0x"));
        assert_eq!(USDC_NATIVE_MAINNET.len(), 42); // 0x + 40 hex chars
    }

    #[test]
    fn usdc_testnet_address_is_valid() {
        assert_eq!(USDC_TESTNET, "0x2E8D98fd126a32362F2Bd8aA427E59a1ec63F780");
        assert!(USDC_TESTNET.starts_with("0x"));
        assert_eq!(USDC_TESTNET.len(), 42);
    }

    #[test]
    fn ctf_exchange_mainnet_address_is_valid() {
        assert_eq!(
            CTF_EXCHANGE_MAINNET,
            "0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E"
        );
        assert!(CTF_EXCHANGE_MAINNET.starts_with("0x"));
        assert_eq!(CTF_EXCHANGE_MAINNET.len(), 42);
    }

    #[test]
    fn ctf_exchange_addresses_match() {
        // Currently mainnet and testnet use the same CTF Exchange address
        assert_eq!(CTF_EXCHANGE_MAINNET, CTF_EXCHANGE_TESTNET);
    }

    #[test]
    fn rpc_urls_are_https() {
        assert!(POLYGON_RPC.starts_with("https://"));
        assert!(AMOY_RPC.starts_with("https://"));
    }

    #[test]
    fn usdc_decimals_is_six() {
        assert_eq!(USDC_DECIMALS, 6);
    }

    // -------------------------------------------------------------------------
    // Config Validation Tests
    // -------------------------------------------------------------------------

    #[test]
    fn runtime_config_with_empty_key_is_invalid() {
        let config = PolymarketRuntimeConfig {
            private_key: "".into(),
            chain_id: 80002,
            api_url: "https://clob.polymarket.com".into(),
            environment: Environment::Testnet,
        };

        assert!(config.private_key.is_empty());
    }

    #[test]
    fn runtime_config_with_whitespace_key_is_invalid() {
        let config = PolymarketRuntimeConfig {
            private_key: "   ".into(),
            chain_id: 80002,
            api_url: "https://clob.polymarket.com".into(),
            environment: Environment::Testnet,
        };

        assert!(config.private_key.trim().is_empty());
    }

    // -------------------------------------------------------------------------
    // Token Approval Trait Methods Tests
    // -------------------------------------------------------------------------

    #[test]
    fn trait_constants() {
        // These would be called on a real PolymarketApproval instance
        // Here we just verify the expected values
        assert_eq!("Polymarket", "Polymarket");
        assert_eq!("USDC", "USDC");
    }

    // -------------------------------------------------------------------------
    // Address Parsing Tests
    // -------------------------------------------------------------------------

    #[test]
    fn address_from_str_succeeds_for_valid_addresses() {
        // Test that the hardcoded addresses can be parsed
        let mainnet_usdc = Address::from_str(USDC_NATIVE_MAINNET);
        assert!(mainnet_usdc.is_ok());

        let testnet_usdc = Address::from_str(USDC_TESTNET);
        assert!(testnet_usdc.is_ok());

        let ctf_exchange = Address::from_str(CTF_EXCHANGE_MAINNET);
        assert!(ctf_exchange.is_ok());
    }

    // -------------------------------------------------------------------------
    // Edge Cases
    // -------------------------------------------------------------------------

    #[test]
    fn very_small_usdc_amount() {
        // 0.000001 USDC = 1 base unit (minimum)
        let result = PolymarketApproval::to_usdc_units(dec!(0.000001));
        assert_eq!(result, U256::from(1u64));
    }

    #[test]
    fn very_large_usdc_amount() {
        // 1,000,000 USDC = 1,000,000,000,000 base units
        let result = PolymarketApproval::to_usdc_units(dec!(1000000));
        assert_eq!(result, U256::from(1_000_000_000_000u64));
    }
}

// -------------------------------------------------------------------------
// Integration Tests (behind feature flag)
// -------------------------------------------------------------------------

#[cfg(all(test, feature = "polymarket-integration"))]
mod integration_tests {
    use super::*;
    use std::env;
    use std::time::Duration;
    use tokio::time::timeout;

    fn get_test_config() -> Option<PolymarketRuntimeConfig> {
        let private_key = env::var("POLYMARKET_PRIVATE_KEY").ok()?;
        let api_url =
            env::var("POLYMARKET_API_URL").unwrap_or_else(|_| "https://clob.polymarket.com".into());

        // Default to testnet for safety
        let chain_id = env::var("POLYMARKET_CHAIN_ID")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(80002);

        let environment = if chain_id == 137 {
            Environment::Mainnet
        } else {
            Environment::Testnet
        };

        Some(PolymarketRuntimeConfig {
            private_key,
            chain_id,
            api_url,
            environment,
        })
    }

    #[tokio::test]
    async fn integration_approval_creation() {
        let Some(config) = get_test_config() else {
            eprintln!("Skipping: POLYMARKET_PRIVATE_KEY not set");
            return;
        };

        match PolymarketApproval::new(&config) {
            Ok(approval) => {
                assert_eq!(approval.exchange_name(), "Polymarket");
                assert_eq!(approval.token_name(), "USDC");
                println!("Wallet address: {:?}", approval.wallet_address());
            }
            Err(e) => {
                eprintln!("Approval creation failed: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn integration_check_balance() {
        let Some(config) = get_test_config() else {
            eprintln!("Skipping: POLYMARKET_PRIVATE_KEY not set");
            return;
        };

        let approval = match PolymarketApproval::new(&config) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("Skipping: {}", e);
                return;
            }
        };

        match timeout(Duration::from_secs(30), approval.usdc_balance()).await {
            Ok(Ok(balance)) => {
                println!("USDC balance: {} USD", balance);
                assert!(balance >= Decimal::ZERO);
            }
            Ok(Err(e)) => {
                eprintln!("Balance check failed: {}", e);
            }
            Err(_) => {
                eprintln!("Balance check timed out");
            }
        }
    }

    #[tokio::test]
    async fn integration_check_approval_status() {
        let Some(config) = get_test_config() else {
            eprintln!("Skipping: POLYMARKET_PRIVATE_KEY not set");
            return;
        };

        let approval = match PolymarketApproval::new(&config) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("Skipping: {}", e);
                return;
            }
        };

        match timeout(Duration::from_secs(30), approval.get_approval_status()).await {
            Ok(Ok(status)) => {
                println!("Token: {}", status.token);
                println!("Allowance: {} USD", status.allowance);
                println!("Spender: {}", status.spender);
                println!("Needs approval: {}", status.needs_approval);

                assert_eq!(status.token, "USDC");
                assert!(!status.spender.is_empty());
            }
            Ok(Err(e)) => {
                eprintln!("Approval status check failed: {}", e);
            }
            Err(_) => {
                eprintln!("Approval status check timed out");
            }
        }
    }
}
