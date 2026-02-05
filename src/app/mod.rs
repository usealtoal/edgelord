//! Application layer - orchestration, configuration, and shared state.

mod config;
mod orchestrator;
mod state;
pub mod wallet;

// Export spawn_execution for testing
pub use orchestrator::execution::spawn_execution;

pub use config::{
    ClusterDetectionConfig, Config, ConnectionPoolConfig, DedupStrategyConfig, Environment,
    Exchange, ExchangeSpecificConfig, GovernorAppConfig, InferenceConfig, LatencyTargetsConfig,
    LlmConfig, LlmProvider, LoggingConfig, NetworkConfig, OutcomeBonusConfig, PolymarketConfig,
    PolymarketConnectionConfig, PolymarketDedupConfig, PolymarketFilterConfig,
    PolymarketScoringConfig, Profile, ReconnectionConfig, ResourceConfig, RiskConfig,
    ScalingAppConfig, ScoringWeightsConfig, StrategiesConfig, TelegramAppConfig, WalletConfig,
};
pub use state::{AppState, RiskLimits};
pub use wallet::{ApprovalOutcome, WalletApprovalStatus, WalletService};

use crate::error::Result;
use orchestrator::Orchestrator;

/// Main application entry point.
///
/// Provides a clean public API for the CLI layer while delegating
/// to internal orchestration components.
pub struct App;

impl App {
    /// Run the main application with the given configuration.
    pub async fn run(config: Config) -> Result<()> {
        Orchestrator::run(config).await
    }
}
