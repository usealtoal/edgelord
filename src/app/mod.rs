//! Application layer - orchestration, configuration, and shared state.

mod config;
mod orchestrator;
mod state;

pub use config::{Config, LoggingConfig, NetworkConfig, StrategiesConfig, WalletConfig};
pub use orchestrator::App;
pub use state::{AppState, RiskLimits};
