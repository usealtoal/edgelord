//! Application layer - orchestration, configuration, and shared state.

mod config;
mod orchestrator;
mod state;
pub mod status;

pub use config::{
    Config, ConnectionPoolConfig, DedupStrategyConfig, Environment, Exchange,
    ExchangeSpecificConfig, GovernorAppConfig, LatencyTargetsConfig, LoggingConfig, NetworkConfig,
    OutcomeBonusConfig, PolymarketConfig, PolymarketConnectionConfig, PolymarketDedupConfig,
    PolymarketFilterConfig, PolymarketScoringConfig, Profile, ReconnectionConfig, ResourceConfig,
    RiskConfig, ScalingAppConfig, ScoringWeightsConfig, StrategiesConfig, TelegramAppConfig,
    WalletConfig,
};
pub use orchestrator::App;
pub use state::{AppState, RiskLimits};
