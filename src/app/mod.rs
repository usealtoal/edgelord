//! Application layer - orchestration, configuration, and shared state.

mod config;
mod orchestrator;
mod state;

pub mod statistics;
pub mod status;
pub mod wallet;

pub use config::{
    ClusterDetectionConfig, Config, ConnectionPoolConfig, DedupStrategyConfig, Environment,
    Exchange, ExchangeSpecificConfig, GovernorAppConfig, InferenceConfig, LatencyTargetsConfig,
    LlmConfig, LlmProvider, LoggingConfig, NetworkConfig, OutcomeBonusConfig, PolymarketConfig,
    PolymarketConnectionConfig, PolymarketDedupConfig, PolymarketFilterConfig, PolymarketHttpConfig,
    PolymarketScoringConfig, Profile, ReconnectionConfig, ResourceConfig, RiskConfig,
    ScalingAppConfig, ScoringWeightsConfig, StrategiesConfig, TelegramAppConfig, WalletConfig,
};
pub use orchestrator::{health_check, HealthCheck, HealthReport, HealthStatus};
pub use state::{AppState, RiskLimits};
pub use wallet::{ApprovalOutcome, SweepOutcome, WalletApprovalStatus, WalletService};

use crate::error::Result;
use orchestrator::Orchestrator;
use tokio::sync::watch;

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

    /// Run the application with a shutdown signal.
    pub async fn run_with_shutdown(
        config: Config,
        shutdown: watch::Receiver<bool>,
    ) -> Result<()> {
        Orchestrator::run_with_shutdown(config, shutdown).await
    }
}
