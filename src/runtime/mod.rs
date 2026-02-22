//! Orchestration, configuration, and wiring.
//!
//! This module contains the runtime components that wire the application together:
//! - `config` - Application configuration loading and validation
//! - `orchestrator` - Main application orchestration loop
//! - `state` - Shared application state
//! - `governor` - Adaptive performance governor
//! - `subscription` - Subscription management
//! - `cache` - Book and cluster caching
//! - `exchange` - Exchange abstraction layer

// Allow many arguments for handler functions that coordinate multiple services
#![allow(clippy::too_many_arguments)]

mod builder;
pub mod cache;
pub mod config;
pub mod exchange;
mod execution;
pub mod governor;
mod handler;
mod monitoring;
mod orchestrator;
mod orchestrator_builder;
mod resource;
mod scaling;
mod score;
mod state;
pub mod subscription;

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

// Re-export cache types
pub use cache::{BookCache, BookUpdate, ClusterCache, PositionTracker};

// Re-export exchange types
pub use exchange::{
    ApprovalResult, ApprovalStatus, ArbitrageExecutor, ConnectionPool, DedupConfig, DedupStrategy,
    ExchangeConfig, ExchangeFactory, ExecutionResult, MarketDataStream, MarketEvent, MarketFetcher,
    MarketFilter, MarketFilterConfig, MarketInfo, MarketScorer, MessageDeduplicator, OrderExecutor,
    OrderRequest, OrderSide, OutcomeInfo, ReconnectingDataStream, StreamFactory, TokenApproval,
};

// Re-export subscription types
pub use subscription::{ConnectionEvent, PrioritySubscriptionManager, SubscriptionManager};

// Re-export infrastructure types (monitoring, resource management, scaling)
pub use monitoring::PoolStats;
pub use resource::ResourceBudget;
pub use scaling::ScalingRecommendation;
pub use score::{MarketScore, ScoreFactors, ScoreWeights};
