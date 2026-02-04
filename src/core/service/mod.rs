//! Cross-cutting services - risk management, notifications, etc.

mod governor;
mod notifier;
mod risk;
mod subscription;

#[cfg(feature = "telegram")]
mod telegram;

pub use governor::{
    AdaptiveGovernor, GovernorConfig, LatencyMetrics, LatencyTargets, ScalingConfig,
};
pub use notifier::{
    Event, ExecutionEvent, LogNotifier, Notifier, NotifierRegistry, NullNotifier, OpportunityEvent,
    RiskEvent, SummaryEvent,
};
pub use risk::{RiskCheckResult, RiskManager};
pub use subscription::{ConnectionEvent, SubscriptionManager};

#[cfg(feature = "telegram")]
pub use telegram::{TelegramConfig, TelegramNotifier};
