//! Orchestration, configuration, and wiring.
//!
//! This module contains the runtime components that wire the application together:
//! - `config` - Application configuration loading and validation
//! - `orchestrator` - Main application orchestration loop
//! - `state` - Shared application state
//! - `governor` - Adaptive performance governor

// Allow many arguments for handler functions that coordinate multiple services
#![allow(clippy::too_many_arguments)]

mod builder;
pub mod config;
mod execution;
pub mod governor;
mod handler;
mod orchestrator;
mod orchestrator_builder;
mod state;

pub use builder::Builder;
pub use config::{
    ClusterDetectionConfig, Config, ConnectionPoolConfig, DedupStrategyConfig, Environment,
    Exchange, ExchangeSpecificConfig, GovernorAppConfig, InferenceConfig, LatencyTargetsConfig,
    LlmConfig, LlmProvider, LoggingConfig, NetworkConfig, OutcomeBonusConfig, PolymarketConfig,
    PolymarketDedupConfig, PolymarketFilterConfig, PolymarketHttpConfig, PolymarketScoringConfig,
    Profile, ReconnectionConfig, ResourceConfig, RiskConfig, ScalingAppConfig,
    ScoringWeightsConfig, StrategiesConfig, TelegramAppConfig, WalletConfig,
};
pub use governor::{
    AdaptiveGovernor, GovernorConfig, LatencyGovernor, LatencyMetrics, LatencyTargets,
    ScalingConfig,
};
pub use orchestrator::{
    health_check, process_market_event, EventProcessingContext, HealthCheck, HealthReport,
    HealthStatus, Orchestrator,
};
pub use state::{AppState, RiskLimitKind, RiskLimitUpdateError, RiskLimits};
