//! Strategy abstraction for arbitrage detection.
//!
//! This module provides a pluggable strategy system supporting multiple
//! detection algorithms:
//!
//! - **`SingleCondition`**: YES + NO < $1 (26.7% of historical profits)
//! - **`MarketRebalancing`**: Sum of outcomes < $1 (73.1% of historical profits)
//! - **Combinatorial**: Frank-Wolfe + ILP for correlated markets (0.24%)
//!
//! # Architecture
//!
//! Each strategy implements the [`Strategy`] trait, which defines:
//! - `name()` - Unique identifier for logging/config
//! - `applies_to()` - Whether strategy should run for a market
//! - `detect()` - Core detection logic
//!
//! The [`StrategyRegistry`] manages enabled strategies and coordinates detection.
//!
//! # Example
//!
//! ```ignore
//! use edgelord::core::strategy::{StrategyRegistry, SingleConditionStrategy};
//!
//! let mut registry = StrategyRegistry::new();
//! registry.register(Box::new(SingleConditionStrategy::new(Default::default())));
//!
//! let opportunities = registry.detect_all(&ctx);
//! ```

pub mod combinatorial;
pub mod condition;
mod context;
pub mod rebalancing;
mod registry;
mod traits;

pub use combinatorial::{CombinatorialConfig, CombinatorialStrategy};
pub use condition::{SingleConditionConfig, SingleConditionStrategy};
pub use context::{DetectionContext, DetectionResult, MarketContext};
pub use rebalancing::{
    MarketRebalancingConfig, MarketRebalancingStrategy, RebalancingLeg, RebalancingOpportunity,
};
pub use registry::{StrategyRegistry, StrategyRegistryBuilder};
pub use traits::Strategy;
