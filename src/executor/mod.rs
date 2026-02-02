//! Trade execution module.

mod orders;
mod positions;

// Re-exports (unused until executor is wired in)
#[allow(unused_imports)]
pub use orders::{ExecutionResult, OrderExecutor};
#[allow(unused_imports)]
pub use positions::{Position, PositionId, PositionLeg, PositionStatus, PositionTracker};
