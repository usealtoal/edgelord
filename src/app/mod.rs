//! Application layer - orchestration, configuration, and shared state.

mod config;
mod orchestrator;
mod state;
pub mod status_file;

pub use config::{
    Config, Exchange, LoggingConfig, NetworkConfig, PolymarketConfig, RiskConfig,
    StrategiesConfig, TelegramAppConfig, WalletConfig,
};
pub use orchestrator::App;
pub use state::{AppState, RiskLimits};
