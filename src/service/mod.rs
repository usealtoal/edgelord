//! Cross-cutting services - risk management, notifications, etc.

mod notifier;
mod risk;

pub use notifier::{
    Event, ExecutionEvent, LogNotifier, Notifier, NotifierRegistry, NullNotifier,
    OpportunityEvent, RiskEvent, SummaryEvent,
};
pub use risk::{RiskCheckResult, RiskManager};
