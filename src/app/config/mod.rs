//! Application configuration loading and validation.
//!
//! Configuration is loaded from a TOML file with environment variable overrides
//! for sensitive values like `WALLET_PRIVATE_KEY`.

use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::error::{ConfigError, Result};

// Submodules
mod inference;
mod llm;
mod logging;
mod polymarket;
mod profile;
mod service;
mod strategy;

// Re-export all public types from submodules
pub use inference::InferenceConfig;
pub use llm::{AnthropicConfig, LlmConfig, LlmProvider, OpenAiConfig};
pub use logging::LoggingConfig;
pub use polymarket::{
    DedupStrategyConfig, Environment, OutcomeBonusConfig, PolymarketConfig,
    PolymarketConnectionConfig, PolymarketDedupConfig, PolymarketFilterConfig,
    PolymarketScoringConfig, ScoringWeightsConfig,
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
    /// Path to the status file for external monitoring.
    /// Set to enable status file writing (e.g., "/var/run/edgelord/status.json").
    #[serde(default)]
    pub status_file: Option<PathBuf>,
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
}

impl Config {
    #[allow(clippy::result_large_err)]
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(ConfigError::ReadFile)?;

        let mut config: Self = toml::from_str(&content).map_err(ConfigError::Parse)?;

        // Load private key from environment variable (never from config file for security)
        config.wallet.private_key = std::env::var("WALLET_PRIVATE_KEY").ok();

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

