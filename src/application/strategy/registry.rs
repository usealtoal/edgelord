//! Strategy registry for managing and executing detection algorithms.

use std::sync::Arc;

use crate::application::cache::cluster::ClusterCache;
use crate::domain::{market::MarketRegistry, opportunity::Opportunity};
use crate::port::{
    inbound::strategy::DetectionContext, inbound::strategy::MarketContext,
    inbound::strategy::Strategy, inbound::strategy::StrategyEngine,
    outbound::solver::ProjectionSolver,
};

use super::combinatorial::{CombinatorialConfig, CombinatorialStrategy};
use super::market_rebalancing::{MarketRebalancingConfig, MarketRebalancingStrategy};
use super::single_condition::{SingleConditionConfig, SingleConditionStrategy};

/// Registry of enabled arbitrage detection strategies.
///
/// Manages a collection of strategies and coordinates running applicable
/// strategies during opportunity detection. Strategies are executed in
/// registration order.
///
/// Use [`StrategyRegistryBuilder`] for convenient construction from configuration.
///
/// # Example
///
/// ```ignore
/// let registry = StrategyRegistry::builder()
///     .single_condition(config.single_condition)
///     .market_rebalancing(config.market_rebalancing)
///     .build();
/// ```
#[derive(Default)]
pub struct StrategyRegistry {
    /// Registered strategies in execution order.
    strategies: Vec<Box<dyn Strategy>>,
}

impl StrategyRegistry {
    /// Create a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a builder for constructing a registry from configuration.
    #[must_use]
    pub fn builder() -> StrategyRegistryBuilder {
        StrategyRegistryBuilder::new()
    }

    /// Register a strategy to be run during detection.
    ///
    /// Strategies are executed in the order they are registered.
    pub fn register(&mut self, strategy: Box<dyn Strategy>) {
        self.strategies.push(strategy);
    }

    /// Return a slice of all registered strategies.
    #[must_use]
    pub fn strategies(&self) -> &[Box<dyn Strategy>] {
        &self.strategies
    }

    /// Return the number of registered strategies.
    #[must_use]
    pub fn len(&self) -> usize {
        self.strategies.len()
    }

    /// Return true if no strategies are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.strategies.is_empty()
    }

    /// Inject the market registry into strategies that require it.
    ///
    /// Must be called after the [`MarketRegistry`] is built, as markets are
    /// typically fetched after strategy construction during orchestrator setup.
    pub fn set_registry(&mut self, registry: Arc<MarketRegistry>) {
        for strategy in &mut self.strategies {
            strategy.set_market_registry(registry.clone());
        }
    }

    /// Run all applicable strategies and collect detected opportunities.
    ///
    /// Only strategies where [`Strategy::applies_to`] returns true for the
    /// current market context are executed.
    #[must_use]
    pub fn detect_all(&self, ctx: &dyn DetectionContext) -> Vec<Opportunity> {
        let market_ctx = ctx.market_context();
        self.strategies
            .iter()
            .filter(|s| s.applies_to(&market_ctx))
            .flat_map(|s| s.detect(ctx))
            .collect()
    }

    /// Run all applicable strategies with an explicit market context.
    ///
    /// Useful when the market context is known ahead of time and does not
    /// need to be derived from the detection context.
    #[must_use]
    pub fn detect_all_with_context(
        &self,
        ctx: &dyn DetectionContext,
        market_ctx: &MarketContext,
    ) -> Vec<Opportunity> {
        self.strategies
            .iter()
            .filter(|s| s.applies_to(market_ctx))
            .flat_map(|s| s.detect(ctx))
            .collect()
    }
}

impl StrategyEngine for StrategyRegistry {
    fn strategy_names(&self) -> Vec<&'static str> {
        self.strategies
            .iter()
            .map(|strategy| strategy.name())
            .collect()
    }

    fn set_market_registry(&mut self, registry: Arc<MarketRegistry>) {
        self.set_registry(registry);
    }

    fn detect_opportunities(&self, ctx: &dyn DetectionContext) -> Vec<Opportunity> {
        self.detect_all(ctx)
    }
}

/// Builder for constructing a [`StrategyRegistry`] from configuration.
///
/// Provides a fluent API for enabling strategies with their configurations
/// and injecting required dependencies.
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
    /// Cluster cache for combinatorial strategy (optional).
    cluster_cache: Option<Arc<ClusterCache>>,
    /// Projection solver for combinatorial strategy (optional).
    projection_solver: Option<Arc<dyn ProjectionSolver>>,
    /// Single-condition strategy configuration (optional).
    single_condition: Option<SingleConditionConfig>,
    /// Market rebalancing strategy configuration (optional).
    market_rebalancing: Option<MarketRebalancingConfig>,
    /// Combinatorial strategy configuration (optional).
    combinatorial: Option<CombinatorialConfig>,
}

impl StrategyRegistryBuilder {
    /// Create a new empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the cluster cache for the combinatorial strategy.
    #[must_use]
    pub fn cluster_cache(mut self, cache: Arc<ClusterCache>) -> Self {
        self.cluster_cache = Some(cache);
        self
    }

    /// Set the projection solver for the combinatorial strategy.
    #[must_use]
    pub fn projection_solver(mut self, solver: Arc<dyn ProjectionSolver>) -> Self {
        self.projection_solver = Some(solver);
        self
    }

    /// Enable the single-condition strategy with the given configuration.
    #[must_use]
    pub fn single_condition(mut self, config: SingleConditionConfig) -> Self {
        self.single_condition = Some(config);
        self
    }

    /// Enable the market rebalancing strategy with the given configuration.
    #[must_use]
    pub fn market_rebalancing(mut self, config: MarketRebalancingConfig) -> Self {
        self.market_rebalancing = Some(config);
        self
    }

    /// Enable the combinatorial strategy with the given configuration.
    #[must_use]
    pub fn combinatorial(mut self, config: CombinatorialConfig) -> Self {
        self.combinatorial = Some(config);
        self
    }

    /// Build the registry with all configured strategies.
    ///
    /// Strategies are registered in order: single-condition, market rebalancing,
    /// then combinatorial.
    #[must_use]
    pub fn build(self) -> StrategyRegistry {
        let cluster_cache = self.cluster_cache;
        let projection_solver = self.projection_solver;
        let mut registry = StrategyRegistry::new();

        if let Some(config) = self.single_condition {
            registry.register(Box::new(SingleConditionStrategy::new(config)));
        }

        if let Some(config) = self.market_rebalancing {
            registry.register(Box::new(MarketRebalancingStrategy::new(config)));
        }

        if let Some(config) = self.combinatorial {
            let mut strategy = CombinatorialStrategy::new(config);
            if let Some(cache) = cluster_cache {
                strategy.set_cache(cache);
            }
            if let Some(solver) = projection_solver {
                strategy.set_projection_solver(solver);
            }
            registry.register(Box::new(strategy));
        }

        registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::inbound::strategy::MarketContext;

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

        fn detect(&self, _ctx: &dyn DetectionContext) -> Vec<Opportunity> {
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
