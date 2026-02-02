use serde::Deserialize;
use std::path::Path;
use tracing_subscriber::{fmt, EnvFilter};

use crate::detector::DetectorConfig;
use crate::error::{Error, Result};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub network: NetworkConfig,
    pub logging: LoggingConfig,
    #[serde(default)]
    pub detector: DetectorConfig,
}

#[derive(Debug, Deserialize)]
pub struct NetworkConfig {
    pub ws_url: String,
    pub api_url: String,
}

#[derive(Debug, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
}

#[allow(clippy::result_large_err)]
impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::Config(format!("Failed to read config file: {}", e)))?;

        let config: Config = toml::from_str(&content)
            .map_err(|e| Error::Config(format!("Failed to parse config: {}", e)))?;

        config.validate()?;

        Ok(config)
    }

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
            },
            logging: LoggingConfig {
                level: "info".into(),
                format: "pretty".into(),
            },
            detector: DetectorConfig::default(),
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
