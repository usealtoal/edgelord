//! Infrastructure bootstrap helpers for runtime wiring.
//!
//! This module provides a thin composition root that delegates to factory modules
//! for building individual components.

// Re-export factory functions for backwards compatibility and convenience.
// Consumers can use either `bootstrap::build_*` or `factory::*::build_*`.

pub use crate::infrastructure::factory::executor::build_executor as init_executor;
pub use crate::infrastructure::factory::inference::{build_cluster_cache, build_inferrer};
pub use crate::infrastructure::factory::llm::build_llm_client;
pub use crate::infrastructure::factory::notifier::build_notifier_registry;
pub use crate::infrastructure::factory::persistence::build_stats_recorder as init_stats_recorder;
pub use crate::infrastructure::factory::solver::build_projection_solver;
pub use crate::infrastructure::factory::strategy::build_strategy_registry;
