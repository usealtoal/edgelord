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

use std::sync::Arc;

use crate::core::domain::{MarketRegistry, Opportunity};

pub use combinatorial::{CombinatorialConfig, CombinatorialStrategy};
pub use condition::{SingleConditionConfig, SingleConditionStrategy};
pub use context::{DetectionContext, DetectionResult, MarketContext};
pub use rebalancing::{
    MarketRebalancingConfig, MarketRebalancingStrategy, RebalancingLeg, RebalancingOpportunity,
};
pub use registry::{StrategyRegistry, StrategyRegistryBuilder};

/// A detection strategy that finds arbitrage opportunities.
///
/// Strategies encapsulate specific detection algorithms. Each strategy
/// can be configured independently and may apply to different market types.
pub trait Strategy: Send + Sync {
    /// Unique identifier for this strategy.
    ///
    /// Used in configuration and logging.
    fn name(&self) -> &'static str;

    /// Check if this strategy should run for a given market context.
    ///
    /// For example, single-condition only applies to binary markets,
    /// while market rebalancing applies to multi-outcome markets.
    fn applies_to(&self, ctx: &MarketContext) -> bool;

    /// Detect opportunities given current market state.
    ///
    /// Returns all found opportunities (may be empty).
    fn detect(&self, ctx: &DetectionContext) -> Vec<Opportunity>;

    /// Optional: provide warm-start hint from previous detection.
    ///
    /// Strategies can use this to speed up iterative algorithms
    /// (e.g., Frank-Wolfe can reuse previous solution).
    fn warm_start(&mut self, _previous: &DetectionResult) {}

    /// Optional: inject the market registry for strategies that need it.
    ///
    /// Called by the orchestrator after the registry is built. Strategies
    /// that don't need it can ignore this (default no-op).
    fn set_market_registry(&mut self, _registry: Arc<MarketRegistry>) {}
}
