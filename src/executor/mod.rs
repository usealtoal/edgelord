//! Trade execution module.

mod orders;

// Re-exports (unused until executor is wired in)
#[allow(unused_imports)]
pub use orders::{ExecutionResult, OrderExecutor};

// Position types are now in domain module - re-export for backward compatibility
#[allow(unused_imports)]
pub use edgelord::domain::{Position, PositionId, PositionLeg, PositionStatus, PositionTracker};
