//! Cross-cutting services - risk management, messaging, etc.

pub mod cluster;
pub mod inference;
mod messaging;
pub mod position;
mod risk;
pub mod statistics;
mod subscription;

// Re-export governor from runtime module for backward compatibility
pub use crate::runtime::governor::{
    AdaptiveGovernor, GovernorConfig, LatencyGovernor, LatencyMetrics, LatencyTargets,
    ScalingConfig,
};
pub use messaging::{
    Event, ExecutionEvent, LogNotifier, Notifier, NotifierRegistry, NullNotifier, OpportunityEvent,
    RelationDetail, RelationsEvent, RiskEvent, SummaryEvent,
};
pub use position::{CloseReason, CloseResult, MarketSettledEvent, PositionManager};
pub use risk::{RiskCheckResult, RiskManager};
pub use statistics::{
    OpportunitySummary, RecordedOpportunity, StatsRecorder, StatsSummary, TradeCloseEvent,
    TradeLeg, TradeOpenEvent,
};
pub use subscription::{ConnectionEvent, PrioritySubscriptionManager, SubscriptionManager};

pub use inference::{
    run_full_inference, InferenceResult, InferenceService, InferenceServiceHandle,
};

#[cfg(feature = "telegram")]
pub use messaging::{RuntimeStats, TelegramConfig, TelegramNotifier};
