//! Application configuration loading and validation.
//!
//! Configuration is loaded from a TOML file with environment variable overrides
//! for sensitive values like `WALLET_PRIVATE_KEY`.

use rust_decimal::Decimal;
use serde::Deserialize;
use std::fs;
use std::path::Path;

use crate::error::{ConfigError, Result};

// Submodules
mod cluster;
mod inference;
mod llm;
mod logging;
mod polymarket;
mod profile;
mod service;
mod strategy;

// Re-export all public types from submodules
pub use cluster::ClusterDetectionConfig;
pub use inference::InferenceConfig;
// Note: AnthropicConfig and OpenAiConfig are exported for programmatic config construction
#[allow(unused_imports)]
pub use llm::{AnthropicConfig, LlmConfig, LlmProvider, OpenAiConfig};
pub use logging::LoggingConfig;
pub use polymarket::{
    DedupStrategyConfig, Environment, OutcomeBonusConfig, PolymarketConfig, PolymarketDedupConfig,
    PolymarketFilterConfig, PolymarketHttpConfig, PolymarketScoringConfig, ScoringWeightsConfig,
};
pub use profile::{Profile, ResourceConfig};
pub use service::{
    ConnectionPoolConfig, GovernorAppConfig, LatencyTargetsConfig, ReconnectionConfig, RiskConfig,
    ScalingAppConfig, TelegramAppConfig, WalletConfig,
};
pub use strategy::StrategiesConfig;

/// Supported exchanges.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Exchange {
    #[default]
    Polymarket,
}

/// Exchange-specific configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ExchangeSpecificConfig {
    Polymarket(PolymarketConfig),
}

impl Default for ExchangeSpecificConfig {
    fn default() -> Self {
        Self::Polymarket(PolymarketConfig::default())
    }
}

/// Common network configuration returned by exchanges.
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub environment: Environment,
    pub ws_url: String,
    pub api_url: String,
    pub chain_id: u64,
}

/// Main application configuration.
#[derive(Debug, Default, Deserialize)]
pub struct Config {
    /// Application profile for resource allocation.
    #[serde(default)]
    pub profile: Profile,
    /// Resource configuration for adaptive subscription management.
    #[serde(default)]
    pub resources: ResourceConfig,
    /// Which exchange to connect to.
    #[serde(default)]
    pub exchange: Exchange,
    /// Exchange-specific configuration.
    #[serde(default, alias = "polymarket")]
    pub exchange_config: ExchangeSpecificConfig,
    pub logging: LoggingConfig,
    #[serde(default)]
    pub strategies: StrategiesConfig,
    #[serde(default)]
    pub wallet: WalletConfig,
    #[serde(default)]
    pub risk: RiskConfig,
    #[serde(default)]
    pub telegram: TelegramAppConfig,
    /// Governor configuration for adaptive subscription management.
    #[serde(default)]
    pub governor: GovernorAppConfig,
    /// Dry-run mode: detect opportunities but don't execute trades.
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default)]
    pub reconnection: ReconnectionConfig,
    /// Connection pool configuration for WebSocket shard management.
    #[serde(default)]
    pub connection_pool: ConnectionPoolConfig,
    /// LLM provider configuration.
    #[serde(default)]
    pub llm: LlmConfig,
    /// Relation inference configuration.
    #[serde(default)]
    pub inference: InferenceConfig,
    /// Cluster detection service configuration.
    #[serde(default)]
    pub cluster_detection: ClusterDetectionConfig,
    /// Path to SQLite database file.
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
    #[allow(clippy::result_large_err)]
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(ConfigError::ReadFile)?;

        let mut config: Self = toml::from_str(&content).map_err(ConfigError::Parse)?;

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

    /// Initialize logging with the configured settings.
    pub fn init_logging(&self) {
        self.logging.init();
    }
}
