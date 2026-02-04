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
mod context;
pub mod rebalancing;
pub mod condition;

pub use combinatorial::{CombinatorialConfig, CombinatorialStrategy};
pub use context::{DetectionContext, DetectionResult, MarketContext};
pub use rebalancing::{
    MarketRebalancingConfig, MarketRebalancingStrategy, RebalancingLeg, RebalancingOpportunity,
};
pub use condition::{SingleConditionConfig, SingleConditionStrategy};

use crate::core::domain::Opportunity;

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
///
/// Use [`StrategyRegistryBuilder`] for convenient construction from config.
#[derive(Default)]
pub struct StrategyRegistry {
    strategies: Vec<Box<dyn Strategy>>,
}

impl StrategyRegistry {
    /// Create a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a builder for constructing a registry from config.
    #[must_use]
    pub fn builder() -> StrategyRegistryBuilder {
        StrategyRegistryBuilder::new()
    }

    /// Register a strategy.
    ///
    /// Strategies are run in registration order.
    pub fn register(&mut self, strategy: Box<dyn Strategy>) {
        self.strategies.push(strategy);
    }

    /// Get all registered strategies.
    #[must_use]
    pub fn strategies(&self) -> &[Box<dyn Strategy>] {
        &self.strategies
    }

    /// Number of registered strategies.
    #[must_use]
    pub fn len(&self) -> usize {
        self.strategies.len()
    }

    /// Check if registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.strategies.is_empty()
    }

    /// Run all applicable strategies and collect opportunities.
    ///
    /// Only strategies where `applies_to()` returns true are run.
    #[must_use]
    pub fn detect_all(&self, ctx: &DetectionContext) -> Vec<Opportunity> {
        let market_ctx = ctx.market_context();
        self.strategies
            .iter()
            .filter(|s| s.applies_to(&market_ctx))
            .flat_map(|s| s.detect(ctx))
            .collect()
    }
}

/// Builder for constructing a [`StrategyRegistry`] from configuration.
///
/// # Example
///
/// ```ignore
/// let registry = StrategyRegistry::builder()
///     .cluster_cache(cache)
///     .single_condition(config.single_condition.clone())
///     .market_rebalancing(config.market_rebalancing.clone())
///     .combinatorial(config.combinatorial.clone())
///     .build();
/// ```
#[derive(Default)]
pub struct StrategyRegistryBuilder {
    cluster_cache: Option<std::sync::Arc<crate::core::cache::ClusterCache>>,
    single_condition: Option<SingleConditionConfig>,
    market_rebalancing: Option<MarketRebalancingConfig>,
    combinatorial: Option<CombinatorialConfig>,
}

impl StrategyRegistryBuilder {
    /// Create a new builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the cluster cache for combinatorial strategy.
    #[must_use]
    pub fn cluster_cache(mut self, cache: std::sync::Arc<crate::core::cache::ClusterCache>) -> Self {
        self.cluster_cache = Some(cache);
        self
    }

    /// Enable single-condition strategy with config.
    #[must_use]
    pub fn single_condition(mut self, config: SingleConditionConfig) -> Self {
        self.single_condition = Some(config);
        self
    }

    /// Enable market rebalancing strategy with config.
    #[must_use]
    pub fn market_rebalancing(mut self, config: MarketRebalancingConfig) -> Self {
        self.market_rebalancing = Some(config);
        self
    }

    /// Enable combinatorial strategy with config.
    #[must_use]
    pub fn combinatorial(mut self, config: CombinatorialConfig) -> Self {
        self.combinatorial = Some(config);
        self
    }

    /// Build the registry with all configured strategies.
    #[must_use]
    pub fn build(self) -> StrategyRegistry {
        let mut registry = StrategyRegistry::new();

        if let Some(config) = self.single_condition {
            registry.register(Box::new(SingleConditionStrategy::new(config)));
        }

        if let Some(config) = self.market_rebalancing {
            registry.register(Box::new(MarketRebalancingStrategy::new(config)));
        }

        if let Some(config) = self.combinatorial {
            if config.enabled {
                let mut strategy = CombinatorialStrategy::new(config);
                if let Some(cache) = self.cluster_cache {
                    strategy.set_cache(cache);
                }
                registry.register(Box::new(strategy));
            }
        }

        registry
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
