//! Notification system for alerts and events.
//!
//! This module re-exports from `crate::adapters::notifiers` for backward compatibility.

pub use crate::adapters::notifiers::{
    Event, ExecutionEvent, LogNotifier, Notifier, NotifierRegistry, NullNotifier, OpportunityEvent,
    RelationDetail, RelationsEvent, RiskEvent, SummaryEvent,
};

#[cfg(feature = "telegram")]
pub use crate::adapters::notifiers::{RuntimeStats, TelegramConfig, TelegramNotifier};
