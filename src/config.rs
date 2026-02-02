use serde::Deserialize;
use std::path::Path;
use tracing_subscriber::{fmt, EnvFilter};

use crate::domain::DetectorConfig;
use crate::error::{Error, Result};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub network: NetworkConfig,
    pub logging: LoggingConfig,
    #[serde(default)]
    pub detector: DetectorConfig,
    #[serde(default)]
    pub wallet: WalletConfig,
}

/// Wallet configuration for signing orders.
/// Private key is loaded from WALLET_PRIVATE_KEY env var at runtime (never from config file).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct WalletConfig {
    /// Private key loaded from WALLET_PRIVATE_KEY env var at runtime
    #[serde(skip)]
    pub private_key: Option<String>,
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
fn default_chain_id() -> u64 {
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
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::Config(format!("Failed to read config file: {}", e)))?;

        let mut config: Config = toml::from_str(&content)
            .map_err(|e| Error::Config(format!("Failed to parse config: {}", e)))?;

        // Load private key from environment variable (never from config file for security)
        config.wallet.private_key = std::env::var("WALLET_PRIVATE_KEY").ok();

        config.validate()?;

        Ok(config)
    }

    #[allow(clippy::result_large_err)]
    fn validate(&self) -> Result<()> {
        if self.network.ws_url.is_empty() {
            return Err(Error::Config("ws_url cannot be empty".into()));
        }
        if self.network.api_url.is_empty() {
            return Err(Error::Config("api_url cannot be empty".into()));
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            network: NetworkConfig {
                ws_url: "wss://ws-subscriptions-clob.polymarket.com/ws/market".into(),
                api_url: "https://clob.polymarket.com".into(),
                chain_id: default_chain_id(),
            },
            logging: LoggingConfig {
                level: "info".into(),
                format: "pretty".into(),
            },
            detector: DetectorConfig::default(),
            wallet: WalletConfig::default(),
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
