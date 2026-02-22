use std::sync::Arc;

use crate::domain::{MarketRegistry, Opportunity};
use crate::runtime::cache::ClusterCache;

use super::Strategy;

use super::combinatorial::{CombinatorialConfig, CombinatorialStrategy};
use super::condition::{SingleConditionConfig, SingleConditionStrategy};
use super::context::DetectionContext;
use super::rebalancing::{MarketRebalancingConfig, MarketRebalancingStrategy};

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

    /// Inject the market registry into strategies that need it (e.g. combinatorial).
    ///
    /// This must be called after the [`MarketRegistry`] is built, since markets
    /// are fetched after strategy construction in the orchestrator.
    pub fn set_registry(&mut self, registry: Arc<MarketRegistry>) {
        for strategy in &mut self.strategies {
            strategy.set_market_registry(registry.clone());
        }
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
    cluster_cache: Option<Arc<ClusterCache>>,
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
    pub fn cluster_cache(mut self, cache: Arc<ClusterCache>) -> Self {
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
            let mut strategy = CombinatorialStrategy::new(config);
            if let Some(cache) = self.cluster_cache {
                strategy.set_cache(cache);
            }
            registry.register(Box::new(strategy));
        }

        registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::strategy::{DetectionContext, MarketContext};

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
