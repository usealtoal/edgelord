//! Cross-cutting services - risk management, notifications, etc.

mod risk;

pub use risk::{RiskCheckResult, RiskManager};
