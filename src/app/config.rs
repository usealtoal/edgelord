//! Application configuration loading and validation.
//!
//! Configuration is loaded from a TOML file with environment variable overrides
//! for sensitive values like `WALLET_PRIVATE_KEY`.

use rust_decimal::Decimal;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tracing_subscriber::{fmt, EnvFilter};

use crate::app::state::RiskLimits;
use crate::core::strategy::{
    CombinatorialConfig, MarketRebalancingConfig, SingleConditionConfig,
};
use crate::error::{ConfigError, Result};

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

/// Exchange environment (testnet vs mainnet).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    #[default]
    Testnet,
    Mainnet,
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Testnet => write!(f, "testnet"),
            Self::Mainnet => write!(f, "mainnet"),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
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
    /// Dry-run mode: detect opportunities but don't execute trades.
    #[serde(default)]
    pub dry_run: bool,
    /// Path to the status file for external monitoring.
    /// Set to enable status file writing (e.g., "/var/run/edgelord/status.json").
    #[serde(default)]
    pub status_file: Option<PathBuf>,
    #[serde(default)]
    pub reconnection: ReconnectionConfig,
}

/// Configuration for all detection strategies.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct StrategiesConfig {
    /// Enabled strategy names.
    #[serde(default = "default_enabled_strategies")]
    pub enabled: Vec<String>,

    /// Single-condition strategy config.
    #[serde(default)]
    pub single_condition: SingleConditionConfig,

    /// Market rebalancing strategy config.
    #[serde(default)]
    pub market_rebalancing: MarketRebalancingConfig,

    /// Combinatorial (Frank-Wolfe + ILP) strategy config.
    #[serde(default)]
    pub combinatorial: CombinatorialConfig,
}

fn default_enabled_strategies() -> Vec<String> {
    vec!["single_condition".to_string()]
}

/// Wallet configuration for signing orders.
/// Private key is loaded from `WALLET_PRIVATE_KEY` env var at runtime (never from config file).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct WalletConfig {
    /// Private key loaded from `WALLET_PRIVATE_KEY` env var at runtime
    #[serde(skip)]
    pub private_key: Option<String>,
}

/// Risk management configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct RiskConfig {
    /// Maximum position size per market in dollars.
    #[serde(default = "default_max_position_per_market")]
    pub max_position_per_market: Decimal,
    /// Maximum total exposure across all positions.
    #[serde(default = "default_max_total_exposure")]
    pub max_total_exposure: Decimal,
    /// Minimum profit threshold to execute.
    #[serde(default = "default_min_profit_threshold")]
    pub min_profit_threshold: Decimal,
    /// Maximum slippage tolerance (e.g., 0.02 = 2%).
    #[serde(default = "default_max_slippage")]
    pub max_slippage: Decimal,
}

fn default_max_position_per_market() -> Decimal {
    Decimal::from(1000)
}

fn default_max_total_exposure() -> Decimal {
    Decimal::from(10000)
}

fn default_min_profit_threshold() -> Decimal {
    Decimal::new(5, 2) // $0.05
}

fn default_max_slippage() -> Decimal {
    Decimal::new(2, 2) // 2%
}

const fn default_true() -> bool {
    true
}

/// Telegram notification configuration.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TelegramAppConfig {
    /// Enable telegram notifications.
    #[serde(default)]
    pub enabled: bool,
    /// Send opportunity alerts (can be noisy).
    #[serde(default)]
    pub notify_opportunities: bool,
    /// Send execution alerts.
    #[serde(default = "default_true")]
    pub notify_executions: bool,
    /// Send risk rejection alerts.
    #[serde(default = "default_true")]
    pub notify_risk_rejections: bool,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_position_per_market: default_max_position_per_market(),
            max_total_exposure: default_max_total_exposure(),
            min_profit_threshold: default_min_profit_threshold(),
            max_slippage: default_max_slippage(),
        }
    }
}

/// WebSocket reconnection configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ReconnectionConfig {
    /// Initial delay before first reconnection attempt (milliseconds).
    #[serde(default = "default_initial_delay_ms")]
    pub initial_delay_ms: u64,
    /// Maximum delay between reconnection attempts (milliseconds).
    #[serde(default = "default_max_delay_ms")]
    pub max_delay_ms: u64,
    /// Multiplier applied to delay after each failed attempt.
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,
    /// Maximum consecutive failures before circuit breaker trips.
    #[serde(default = "default_max_consecutive_failures")]
    pub max_consecutive_failures: u32,
    /// Cooldown period after circuit breaker trips (milliseconds).
    #[serde(default = "default_circuit_breaker_cooldown_ms")]
    pub circuit_breaker_cooldown_ms: u64,
}

fn default_initial_delay_ms() -> u64 {
    1000 // 1 second
}

fn default_max_delay_ms() -> u64 {
    60000 // 60 seconds
}

fn default_backoff_multiplier() -> f64 {
    2.0
}

fn default_max_consecutive_failures() -> u32 {
    10
}

fn default_circuit_breaker_cooldown_ms() -> u64 {
    300000 // 5 minutes
}

impl Default for ReconnectionConfig {
    fn default() -> Self {
        Self {
            initial_delay_ms: default_initial_delay_ms(),
            max_delay_ms: default_max_delay_ms(),
            backoff_multiplier: default_backoff_multiplier(),
            max_consecutive_failures: default_max_consecutive_failures(),
            circuit_breaker_cooldown_ms: default_circuit_breaker_cooldown_ms(),
        }
    }
}

impl From<RiskConfig> for RiskLimits {
    fn from(config: RiskConfig) -> Self {
        Self {
            max_position_per_market: config.max_position_per_market,
            max_total_exposure: config.max_total_exposure,
            min_profit_threshold: config.min_profit_threshold,
            max_slippage: config.max_slippage,
        }
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

/// Polymarket exchange configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct PolymarketConfig {
    /// Environment: testnet or mainnet.
    #[serde(default)]
    pub environment: Environment,
    /// WebSocket URL for market data.
    #[serde(default = "default_polymarket_ws_url")]
    pub ws_url: String,
    /// REST API URL.
    #[serde(default = "default_polymarket_api_url")]
    pub api_url: String,
    /// Chain ID: 80002 for Amoy testnet, 137 for Polygon mainnet.
    #[serde(default = "default_polymarket_chain_id")]
    pub chain_id: u64,
}

fn default_polymarket_ws_url() -> String {
    "wss://ws-subscriptions-clob.polymarket.com/ws/market".into()
}

fn default_polymarket_api_url() -> String {
    "https://clob.polymarket.com".into()
}

/// Default chain ID is Amoy testnet (80002) for safety
const fn default_polymarket_chain_id() -> u64 {
    80002
}

impl Default for PolymarketConfig {
    fn default() -> Self {
        Self {
            environment: Environment::default(),
            ws_url: default_polymarket_ws_url(),
            api_url: default_polymarket_api_url(),
            chain_id: default_polymarket_chain_id(),
        }
    }
}


#[derive(Debug, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            exchange: Exchange::default(),
            exchange_config: ExchangeSpecificConfig::default(),
            logging: LoggingConfig {
                level: "info".into(),
                format: "pretty".into(),
            },
            strategies: StrategiesConfig::default(),
            wallet: WalletConfig::default(),
            risk: RiskConfig::default(),
            telegram: TelegramAppConfig::default(),
            dry_run: false,
            status_file: None,
            reconnection: ReconnectionConfig::default(),
        }
    }
}

impl Config {
    pub fn init_logging(&self) {
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(&self.logging.level));

        match self.logging.format.as_str() {
            "json" => {
                fmt().json().with_env_filter(filter).init();
            }
            _ => {
                fmt().with_env_filter(filter).init();
            }
        }
    }
}
