//! Cross-cutting services - risk management, notifications, etc.

pub mod cluster;
mod governor;
mod notification;
mod risk;
pub mod stats;
mod subscription;

pub use governor::{
    AdaptiveGovernor, GovernorConfig, LatencyGovernor, LatencyMetrics, LatencyTargets, ScalingConfig,
};
pub use notification::{
    Event, ExecutionEvent, LogNotifier, Notifier, NotifierRegistry, NullNotifier, OpportunityEvent,
    RiskEvent, SummaryEvent,
};
pub use risk::{RiskCheckResult, RiskManager};
pub use stats::{
    RecordedOpportunity, StatsSummary, StatsRecorder, TradeCloseEvent, TradeOpenEvent, TradeLeg,
};
pub use subscription::{ConnectionEvent, PrioritySubscriptionManager, SubscriptionManager};

#[cfg(feature = "telegram")]
pub use notification::{TelegramConfig, TelegramNotifier};
