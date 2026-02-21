//! Cross-cutting services - risk management, messaging, etc.
//!
//! DEPRECATED: This module is being phased out. Services have been moved to:
//! - `crate::adapters::risk` - Risk management
//! - `crate::adapters::statistics` - Statistics recording
//! - `crate::adapters::position` - Position lifecycle
//! - `crate::adapters::cluster` - Cluster detection
//! - `crate::adapters::inference` - Inference service
//! - `crate::runtime::subscription` - Subscription management

mod messaging;

// Re-export governor from runtime module for backward compatibility
pub use crate::runtime::governor::{
    AdaptiveGovernor, GovernorConfig, LatencyGovernor, LatencyMetrics, LatencyTargets,
    ScalingConfig,
};

// Re-export messaging types (still in core for now)
pub use messaging::{
    Event, ExecutionEvent, LogNotifier, Notifier, NotifierRegistry, NullNotifier, OpportunityEvent,
    RelationDetail, RelationsEvent, RiskEvent, SummaryEvent,
};

// Re-export from new adapter locations
pub use crate::adapters::cluster;
pub use crate::adapters::cluster::{ClusterDetectionConfig as ClusterConfig, ClusterDetector};
pub use crate::adapters::inference::{
    run_full_inference, InferenceResult, InferenceService, InferenceServiceHandle,
};
pub use crate::adapters::position;
pub use crate::adapters::position::{
    CloseReason, CloseResult, MarketSettledEvent, PositionManager,
};
pub use crate::adapters::risk::{RiskCheckResult, RiskManager};

// Re-export statistics module for backward compatibility (used as `statistics::create_recorder()`)
pub use crate::adapters::statistics;
pub use crate::adapters::statistics::{
    OpportunitySummary, RecordedOpportunity, StatsRecorder, StatsSummary, TradeCloseEvent,
    TradeLeg, TradeOpenEvent,
};

// Re-export from new runtime location
pub use crate::runtime::subscription::{
    ConnectionEvent, PrioritySubscriptionManager, SubscriptionManager,
};

#[cfg(feature = "telegram")]
pub use messaging::{RuntimeStats, TelegramConfig, TelegramNotifier};
