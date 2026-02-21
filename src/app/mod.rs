//! Application layer - orchestration, configuration, and shared state.
//!
//! This module re-exports components from [`crate::runtime`] for backward compatibility.

pub mod statistics;
pub mod status;
pub mod wallet;

// Re-export from runtime for backward compatibility
pub use crate::runtime::{
    health_check, process_market_event, ClusterDetectionConfig, Config, ConnectionPoolConfig,
    DedupStrategyConfig, Environment, EventProcessingContext, Exchange, ExchangeSpecificConfig,
    GovernorAppConfig, HealthCheck, HealthReport, HealthStatus, InferenceConfig,
    LatencyTargetsConfig, LlmConfig, LlmProvider, LoggingConfig, NetworkConfig, Orchestrator,
    OutcomeBonusConfig, PolymarketConfig, PolymarketDedupConfig, PolymarketFilterConfig,
    PolymarketHttpConfig, PolymarketScoringConfig, Profile, ReconnectionConfig, ResourceConfig,
    RiskConfig, ScalingAppConfig, ScoringWeightsConfig, StrategiesConfig, TelegramAppConfig,
    WalletConfig,
};
pub use crate::runtime::{AppState, RiskLimitKind, RiskLimitUpdateError, RiskLimits};
pub use wallet::{ApprovalOutcome, SweepOutcome, WalletApprovalStatus, WalletService};

use crate::error::Result;
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
    pub async fn run_with_shutdown(config: Config, shutdown: watch::Receiver<bool>) -> Result<()> {
        Orchestrator::run_with_shutdown(config, shutdown).await
    }
}
