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
    Kalshi,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    /// Which exchange to connect to.
    #[serde(default)]
    pub exchange: Exchange,
    pub network: NetworkConfig,
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
    #[serde(default = "default_status_file")]
    pub status_file: Option<PathBuf>,
}

fn default_status_file() -> Option<PathBuf> {
    Some(PathBuf::from("/var/run/edgelord/status.json"))
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

#[derive(Debug, Deserialize)]
pub struct NetworkConfig {
    pub ws_url: String,
    pub api_url: String,
    /// Chain ID: 80002 for Amoy testnet, 137 for Polygon mainnet
    #[serde(default = "default_chain_id")]
    pub chain_id: u64,
}

/// Default chain ID is Amoy testnet (80002) for safety
const fn default_chain_id() -> u64 {
    80002
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
        if self.network.ws_url.is_empty() {
            return Err(ConfigError::MissingField { field: "ws_url" }.into());
        }
        if self.network.api_url.is_empty() {
            return Err(ConfigError::MissingField { field: "api_url" }.into());
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            exchange: Exchange::default(),
            network: NetworkConfig {
                ws_url: "wss://ws-subscriptions-clob.polymarket.com/ws/market".into(),
                api_url: "https://clob.polymarket.com".into(),
                chain_id: default_chain_id(),
            },
            logging: LoggingConfig {
                level: "info".into(),
                format: "pretty".into(),
            },
            strategies: StrategiesConfig::default(),
            wallet: WalletConfig::default(),
            risk: RiskConfig::default(),
            telegram: TelegramAppConfig::default(),
            dry_run: false,
            status_file: default_status_file(),
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
