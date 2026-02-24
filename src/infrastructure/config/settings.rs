//! Application configuration loading and validation.
//!
//! Provides the main [`Config`] struct that aggregates all application settings.
//! Configuration is loaded from a TOML file with environment variable overrides
//! for sensitive values like `WALLET_PRIVATE_KEY`.
//!
//! # Example
//!
//! ```no_run
//! use edgelord::infrastructure::config::settings::Config;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = Config::load("config.toml")?;
//!     config.init_logging();
//!     Ok(())
//! }
//! ```

use rust_decimal::Decimal;
use serde::Deserialize;
use std::fs;
use std::path::Path;

use super::cluster::ClusterDetectionConfig;
use super::governor::GovernorAppConfig;
use super::llm::LlmConfig;
use super::logging::LoggingConfig;
use super::pool::{ConnectionPoolConfig, ReconnectionConfig};
use super::profile::{Profile, ResourceConfig};
use super::risk::RiskConfig;
use super::strategy::StrategiesConfig;
use super::telegram::TelegramAppConfig;
use super::wallet::WalletConfig;
use crate::adapter::outbound::polymarket::settings::{Environment, PolymarketConfig};
use crate::application::inference::config::InferenceConfig;
use crate::error::{ConfigError, Result};

/// Supported exchange platforms.
///
/// Determines which exchange adapter to use for market data and execution.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Exchange {
    /// Polymarket prediction market exchange.
    #[default]
    Polymarket,
}

/// Exchange-specific configuration variant.
///
/// Contains the configuration settings specific to each supported exchange.
/// The active variant is determined by the `type` field in the TOML config.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ExchangeSpecificConfig {
    /// Polymarket exchange configuration.
    Polymarket(PolymarketConfig),
}

impl Default for ExchangeSpecificConfig {
    fn default() -> Self {
        Self::Polymarket(PolymarketConfig::default())
    }
}

/// Common network configuration returned by exchanges.
///
/// Provides a unified view of network settings regardless of the underlying
/// exchange implementation.
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Target environment (mainnet or testnet).
    pub environment: Environment,
    /// WebSocket URL for streaming market data.
    pub ws_url: String,
    /// REST API URL for order submission and market queries.
    pub api_url: String,
    /// Blockchain chain ID for transaction signing.
    pub chain_id: u64,
}

impl NetworkConfig {
    /// True when the configured environment is explicitly mainnet.
    #[must_use]
    pub fn is_environment_mainnet(&self) -> bool {
        self.environment == Environment::Mainnet
    }

    /// True when the active network is Polygon mainnet.
    #[must_use]
    pub fn is_mainnet(&self) -> bool {
        self.chain_id == 137
    }

    /// True when the active network is Polygon Amoy testnet.
    #[must_use]
    pub fn is_testnet(&self) -> bool {
        self.chain_id == 80002
    }
}

/// Main application configuration.
///
/// Aggregates all configuration settings for the application. Load from a TOML
/// file using [`Config::load`] or parse directly with [`Config::parse_toml`].
#[derive(Debug, Default, Deserialize)]
pub struct Config {
    /// Application profile for resource allocation.
    ///
    /// Determines baseline resource limits. Defaults to [`Profile::Local`].
    #[serde(default)]
    pub profile: Profile,

    /// Resource configuration for adaptive subscription management.
    ///
    /// Fine-grained control over memory and CPU budgets.
    #[serde(default)]
    pub resources: ResourceConfig,

    /// Target exchange platform.
    ///
    /// Determines which exchange adapters to instantiate. Defaults to Polymarket.
    #[serde(default)]
    pub exchange: Exchange,

    /// Exchange-specific configuration settings.
    ///
    /// Contains connection URLs, API settings, and exchange-specific options.
    #[serde(default, alias = "polymarket")]
    pub exchange_config: ExchangeSpecificConfig,

    /// Logging and tracing configuration.
    pub logging: LoggingConfig,

    /// Detection strategy configuration.
    ///
    /// Controls which strategies are enabled and their parameters.
    #[serde(default)]
    pub strategies: StrategiesConfig,

    /// Wallet configuration for order signing.
    ///
    /// Private key is loaded from `WALLET_PRIVATE_KEY` environment variable.
    #[serde(default)]
    pub wallet: WalletConfig,

    /// Risk management limits.
    ///
    /// Controls position sizes, exposure limits, and slippage tolerance.
    #[serde(default)]
    pub risk: RiskConfig,

    /// Telegram notification configuration.
    #[serde(default)]
    pub telegram: TelegramAppConfig,

    /// Governor configuration for adaptive subscription scaling.
    ///
    /// Controls latency targets and scaling behavior.
    #[serde(default)]
    pub governor: GovernorAppConfig,

    /// Enable dry-run mode.
    ///
    /// When true, detects opportunities but does not execute trades.
    /// Defaults to false.
    #[serde(default)]
    pub dry_run: bool,

    /// WebSocket reconnection settings.
    ///
    /// Controls backoff delays and circuit breaker behavior.
    #[serde(default)]
    pub reconnection: ReconnectionConfig,

    /// Connection pool configuration for WebSocket shard management.
    ///
    /// Controls how subscriptions are distributed across connections.
    #[serde(default)]
    pub connection_pool: ConnectionPoolConfig,

    /// LLM provider configuration for inference.
    #[serde(default)]
    pub llm: LlmConfig,

    /// Relation inference configuration.
    ///
    /// Controls LLM-based market relationship detection.
    #[serde(default)]
    pub inference: InferenceConfig,

    /// Cluster detection service configuration.
    #[serde(default)]
    pub cluster_detection: ClusterDetectionConfig,

    /// Path to SQLite database file.
    ///
    /// Defaults to "edgelord.db" in the current directory.
    #[serde(default = "default_database_path")]
    pub database: String,
}

fn default_database_path() -> String {
    "edgelord.db".to_string()
}

fn read_keystore_password() -> Result<String> {
    if let Ok(password) = std::env::var("EDGELORD_KEYSTORE_PASSWORD") {
        return Ok(password);
    }
    if let Ok(path) = std::env::var("EDGELORD_KEYSTORE_PASSWORD_FILE") {
        let contents = fs::read_to_string(path).map_err(ConfigError::ReadFile)?;
        let password = contents.trim().to_string();
        if password.is_empty() {
            return Err(ConfigError::MissingField {
                field: "EDGELORD_KEYSTORE_PASSWORD_FILE",
            }
            .into());
        }
        return Ok(password);
    }

    Err(ConfigError::MissingField {
        field: "EDGELORD_KEYSTORE_PASSWORD",
    }
    .into())
}

#[cfg(feature = "polymarket")]
fn decrypt_keystore_private_key(path: &str, password: &str) -> Result<String> {
    use alloy_signer_local::PrivateKeySigner;

    let signer = PrivateKeySigner::decrypt_keystore(path, password).map_err(|e| {
        ConfigError::InvalidValue {
            field: "keystore_path",
            reason: e.to_string(),
        }
    })?;
    Ok(format!("{:x}", signer.to_bytes()))
}

#[cfg(not(feature = "polymarket"))]
fn decrypt_keystore_private_key(_path: &str, _password: &str) -> Result<String> {
    Err(ConfigError::InvalidValue {
        field: "keystore_path",
        reason: "keystore support requires polymarket feature".to_string(),
    }
    .into())
}

impl Config {
    /// Parse configuration from TOML content.
    ///
    /// Loads the private key from the `WALLET_PRIVATE_KEY` environment variable
    /// or decrypts it from a keystore file if `keystore_path` is configured.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The TOML content is malformed
    /// - Validation fails (e.g., invalid slippage values)
    /// - Keystore decryption fails when using keystore authentication
    #[allow(clippy::result_large_err)]
    pub fn parse_toml(content: &str) -> Result<Self> {
        let mut config: Self = toml::from_str(content).map_err(ConfigError::Parse)?;

        // Load private key from environment variable (never from config file for security)
        config.wallet.private_key = std::env::var("WALLET_PRIVATE_KEY").ok();
        if config.wallet.private_key.is_none() {
            if let Some(ref keystore_path) = config.wallet.keystore_path {
                let password = read_keystore_password()?;
                config.wallet.private_key =
                    Some(decrypt_keystore_private_key(keystore_path, &password)?);
            }
        }

        config.validate()?;

        Ok(config)
    }

    /// Load configuration from a TOML file.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be read
    /// - The TOML content is malformed
    /// - Validation fails
    #[allow(clippy::result_large_err)]
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(ConfigError::ReadFile)?;
        Self::parse_toml(&content)
    }

    /// Validate configuration values.
    ///
    /// Checks that all required fields are present and values are within
    /// acceptable ranges.
    #[allow(clippy::result_large_err)]
    fn validate(&self) -> Result<()> {
        let network = self.network();
        if network.ws_url.is_empty() {
            return Err(ConfigError::MissingField { field: "ws_url" }.into());
        }
        if network.api_url.is_empty() {
            return Err(ConfigError::MissingField { field: "api_url" }.into());
        }
        if self.risk.max_slippage < Decimal::ZERO || self.risk.max_slippage > Decimal::ONE {
            return Err(ConfigError::InvalidValue {
                field: "max_slippage",
                reason: "must be between 0 and 1".to_string(),
            }
            .into());
        }
        if self.risk.max_position_per_market <= Decimal::ZERO {
            return Err(ConfigError::InvalidValue {
                field: "max_position_per_market",
                reason: "must be greater than 0".to_string(),
            }
            .into());
        }
        if self.risk.max_total_exposure <= Decimal::ZERO {
            return Err(ConfigError::InvalidValue {
                field: "max_total_exposure",
                reason: "must be greater than 0".to_string(),
            }
            .into());
        }
        if self.risk.min_profit_threshold < Decimal::ZERO {
            return Err(ConfigError::InvalidValue {
                field: "min_profit_threshold",
                reason: "must be 0 or greater".to_string(),
            }
            .into());
        }

        if self.reconnection.initial_delay_ms == 0 {
            return Err(ConfigError::InvalidValue {
                field: "initial_delay_ms",
                reason: "must be greater than 0".to_string(),
            }
            .into());
        }
        if self.reconnection.max_delay_ms < self.reconnection.initial_delay_ms {
            return Err(ConfigError::InvalidValue {
                field: "max_delay_ms",
                reason: "must be >= initial_delay_ms".to_string(),
            }
            .into());
        }
        if self.reconnection.backoff_multiplier < 1.0 {
            return Err(ConfigError::InvalidValue {
                field: "backoff_multiplier",
                reason: "must be >= 1.0".to_string(),
            }
            .into());
        }
        if self.reconnection.max_consecutive_failures == 0 {
            return Err(ConfigError::InvalidValue {
                field: "max_consecutive_failures",
                reason: "must be greater than 0".to_string(),
            }
            .into());
        }
        if self.reconnection.circuit_breaker_cooldown_ms == 0 {
            return Err(ConfigError::InvalidValue {
                field: "circuit_breaker_cooldown_ms",
                reason: "must be greater than 0".to_string(),
            }
            .into());
        }

        let latency = &self.governor.latency;
        if latency.target_p50_ms == 0
            || latency.target_p95_ms == 0
            || latency.target_p99_ms == 0
            || latency.max_p99_ms == 0
        {
            return Err(ConfigError::InvalidValue {
                field: "latency_targets",
                reason: "latency targets must be greater than 0".to_string(),
            }
            .into());
        }
        if !(latency.target_p50_ms <= latency.target_p95_ms
            && latency.target_p95_ms <= latency.target_p99_ms
            && latency.target_p99_ms <= latency.max_p99_ms)
        {
            return Err(ConfigError::InvalidValue {
                field: "latency_targets",
                reason: "targets must be ordered p50 <= p95 <= p99 <= max_p99".to_string(),
            }
            .into());
        }

        let scaling = &self.governor.scaling;
        if scaling.check_interval_secs == 0
            || scaling.expand_step == 0
            || scaling.contract_step == 0
            || scaling.cooldown_secs == 0
        {
            return Err(ConfigError::InvalidValue {
                field: "scaling_config",
                reason: "interval/steps/cooldown must be greater than 0".to_string(),
            }
            .into());
        }
        if scaling.expand_threshold <= 0.0 || scaling.contract_threshold <= 0.0 {
            return Err(ConfigError::InvalidValue {
                field: "scaling_config",
                reason: "thresholds must be greater than 0".to_string(),
            }
            .into());
        }

        if self.cluster_detection.enabled {
            if self.cluster_detection.debounce_ms == 0 {
                return Err(ConfigError::InvalidValue {
                    field: "debounce_ms",
                    reason: "must be greater than 0".to_string(),
                }
                .into());
            }
            if self.cluster_detection.min_gap < Decimal::ZERO
                || self.cluster_detection.min_gap > Decimal::ONE
            {
                return Err(ConfigError::InvalidValue {
                    field: "min_gap",
                    reason: "must be between 0 and 1".to_string(),
                }
                .into());
            }
            if self.cluster_detection.max_clusters_per_cycle == 0 {
                return Err(ConfigError::InvalidValue {
                    field: "max_clusters_per_cycle",
                    reason: "must be greater than 0".to_string(),
                }
                .into());
            }
            if self.cluster_detection.channel_capacity == 0 {
                return Err(ConfigError::InvalidValue {
                    field: "channel_capacity",
                    reason: "must be greater than 0".to_string(),
                }
                .into());
            }
        }
        Ok(())
    }

    /// Get the network configuration for the active exchange.
    #[must_use]
    pub fn network(&self) -> NetworkConfig {
        match &self.exchange_config {
            ExchangeSpecificConfig::Polymarket(poly) => NetworkConfig {
                environment: poly.environment,
                ws_url: poly.ws_url.clone(),
                api_url: poly.api_url.clone(),
                chain_id: poly.chain_id,
            },
        }
    }

    /// Get Polymarket-specific config if this is a Polymarket exchange.
    #[must_use]
    pub fn polymarket_config(&self) -> Option<&PolymarketConfig> {
        match &self.exchange_config {
            ExchangeSpecificConfig::Polymarket(config) => Some(config),
        }
    }

    /// Switch the active exchange config to mainnet defaults.
    pub fn set_mainnet(&mut self) {
        match &mut self.exchange_config {
            ExchangeSpecificConfig::Polymarket(config) => {
                config.chain_id = 137;
                config.environment = Environment::Mainnet;
            }
        }
    }

    /// Switch the active exchange config to testnet defaults.
    pub fn set_testnet(&mut self) {
        match &mut self.exchange_config {
            ExchangeSpecificConfig::Polymarket(config) => {
                config.chain_id = 80002;
                config.environment = Environment::Testnet;
            }
        }
    }

    /// Initialize logging with the configured settings.
    pub fn init_logging(&self) {
        self.logging.init();
    }
}
