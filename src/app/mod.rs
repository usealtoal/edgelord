//! Application layer - orchestration, configuration, and shared state.

mod config;
mod orchestrator;
mod state;
pub mod status;

pub use config::{
    Config, Environment, Exchange, ExchangeSpecificConfig, LoggingConfig, NetworkConfig,
    OutcomeBonusConfig, PolymarketConfig, PolymarketDedupConfig, PolymarketFilterConfig,
    PolymarketScoringConfig, ReconnectionConfig, RiskConfig, ScoringWeightsConfig,
    StrategiesConfig, TelegramAppConfig, WalletConfig,
};
pub use orchestrator::App;
pub use state::{AppState, RiskLimits};
