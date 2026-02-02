//! Application layer - orchestration, configuration, and shared state.

mod config;
mod orchestrator;

pub use config::{Config, LoggingConfig, NetworkConfig, StrategiesConfig, WalletConfig};
pub use orchestrator::App;
