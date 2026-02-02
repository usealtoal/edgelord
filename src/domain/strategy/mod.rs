//! Strategy abstraction for arbitrage detection.
//!
//! This module provides a pluggable strategy system supporting multiple
//! detection algorithms:
//!
//! - **SingleCondition**: YES + NO < $1 (26.7% of historical profits)
//! - **MarketRebalancing**: Sum of outcomes < $1 (73.1% of historical profits)
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
//! use edgelord::domain::strategy::{StrategyRegistry, SingleConditionStrategy};
//!
//! let mut registry = StrategyRegistry::new();
//! registry.register(Box::new(SingleConditionStrategy::new(Default::default())));
//!
//! let opportunities = registry.detect_all(&ctx);
//! ```

mod context;
pub mod single_condition;
pub mod market_rebalancing;
pub mod combinatorial;

pub use context::{DetectionContext, DetectionResult, MarketContext};
pub use single_condition::{SingleConditionConfig, SingleConditionStrategy};
pub use market_rebalancing::{
    MarketRebalancingConfig, MarketRebalancingStrategy, RebalancingLeg, RebalancingOpportunity,
};
pub use combinatorial::{CombinatorialConfig, CombinatorialStrategy};

use crate::domain::Opportunity;

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
}

/// Registry of enabled strategies.
///
/// The registry manages a collection of strategies and coordinates
/// running them during detection.
#[derive(Default)]
pub struct StrategyRegistry {
    strategies: Vec<Box<dyn Strategy>>,
}

impl StrategyRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a strategy.
    ///
    /// Strategies are run in registration order.
    pub fn register(&mut self, strategy: Box<dyn Strategy>) {
        self.strategies.push(strategy);
    }

    /// Get all registered strategies.
    pub fn strategies(&self) -> &[Box<dyn Strategy>] {
        &self.strategies
    }

    /// Number of registered strategies.
    pub fn len(&self) -> usize {
        self.strategies.len()
    }

    /// Check if registry is empty.
    pub fn is_empty(&self) -> bool {
        self.strategies.is_empty()
    }

    /// Run all applicable strategies and collect opportunities.
    ///
    /// Only strategies where `applies_to()` returns true are run.
    pub fn detect_all(&self, ctx: &DetectionContext) -> Vec<Opportunity> {
        let market_ctx = ctx.market_context();
        self.strategies
            .iter()
            .filter(|s| s.applies_to(&market_ctx))
            .flat_map(|s| s.detect(ctx))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockStrategy {
        name: &'static str,
        applies: bool,
    }

    impl Strategy for MockStrategy {
        fn name(&self) -> &'static str {
            self.name
        }

        fn applies_to(&self, _ctx: &MarketContext) -> bool {
            self.applies
        }

        fn detect(&self, _ctx: &DetectionContext) -> Vec<Opportunity> {
            vec![]
        }
    }

    #[test]
    fn test_registry_new() {
        let registry = StrategyRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_register() {
        let mut registry = StrategyRegistry::new();
        registry.register(Box::new(MockStrategy {
            name: "test",
            applies: true,
        }));

        assert_eq!(registry.len(), 1);
        assert_eq!(registry.strategies()[0].name(), "test");
    }
}
