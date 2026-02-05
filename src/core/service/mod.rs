//! Cross-cutting services - risk management, notifications, etc.

pub mod cluster;
mod governor;
mod notification;
pub mod position;
mod risk;
pub mod statistics;
mod subscription;

pub use governor::{
    AdaptiveGovernor, GovernorConfig, LatencyGovernor, LatencyMetrics, LatencyTargets, ScalingConfig,
};
pub use notification::{
    Event, ExecutionEvent, LogNotifier, Notifier, NotifierRegistry, NullNotifier, OpportunityEvent,
    RiskEvent, SummaryEvent,
};
pub use risk::{RiskCheckResult, RiskManager};
pub use position::{CloseReason, CloseResult, MarketSettledEvent, PositionManager};
pub use statistics::{
    OpportunitySummary, RecordedOpportunity, StatsSummary, StatsRecorder, TradeCloseEvent,
    TradeOpenEvent, TradeLeg,
};
pub use subscription::{ConnectionEvent, PrioritySubscriptionManager, SubscriptionManager};

#[cfg(feature = "telegram")]
pub use notification::{TelegramConfig, TelegramNotifier};
