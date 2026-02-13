//! Combinatorial arbitrage detection using Frank-Wolfe + ILP.
//!
//! This strategy detects arbitrage opportunities across correlated markets
//! where logical dependencies create exploitable mispricings.
//!
//! # Algorithm
//!
//! The Frank-Wolfe algorithm projects market prices onto the arbitrage-free
//! manifold (marginal polytope) using Bregman divergence. Key steps:
//!
//! 1. Start with current market prices θ
//! 2. Compute gradient of Bregman divergence
//! 3. Solve ILP oracle: find vertex minimizing gradient dot product
//! 4. Update toward that vertex
//! 5. Repeat until convergence or iteration limit
//!
//! The gap between θ and the projection μ* indicates arbitrage potential.
//!
//! # Research Background
//!
//! This implements techniques from:
//! - "Arbitrage-Free Combinatorial Market Making via Integer Programming" (2016)
//! - "Unravelling the Probabilistic Forest: Arbitrage in Prediction Markets" (2025)
//!
//! While combinatorial arbitrage captured only 0.24% ($95K) of historical profits,
//! the mathematical infrastructure enables more sophisticated strategies.

pub use crate::core::solver::{bregman_divergence, bregman_gradient, lmsr_cost, lmsr_prices};
pub use crate::core::solver::{FrankWolfe, FrankWolfeConfig, FrankWolfeResult};

use std::sync::Arc;

use rust_decimal::Decimal;
use serde::Deserialize;

use super::{DetectionContext, MarketContext, Strategy};
use crate::core::cache::ClusterCache;
use crate::core::domain::{MarketRegistry, Opportunity};
use crate::core::service::cluster::{ClusterDetectionConfig, ClusterDetector};

/// Configuration for combinatorial strategy.
#[derive(Debug, Clone, Deserialize)]
pub struct CombinatorialConfig {
    /// Maximum Frank-Wolfe iterations per detection.
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,

    /// Convergence tolerance (stop when gap < this).
    #[serde(default = "default_tolerance")]
    pub tolerance: Decimal,

    /// Minimum arbitrage gap to act on.
    #[serde(default = "default_gap_threshold")]
    pub gap_threshold: Decimal,

    /// Enable this strategy.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

const fn default_max_iterations() -> usize {
    20
}

fn default_tolerance() -> Decimal {
    Decimal::new(1, 4) // 0.0001
}

fn default_gap_threshold() -> Decimal {
    Decimal::new(2, 2) // 0.02
}

const fn default_enabled() -> bool {
    false // Disabled by default - requires dependency configuration
}

impl Default for CombinatorialConfig {
    fn default() -> Self {
        Self {
            max_iterations: default_max_iterations(),
            tolerance: default_tolerance(),
            gap_threshold: default_gap_threshold(),
            enabled: default_enabled(),
        }
    }
}

/// Combinatorial arbitrage strategy using Frank-Wolfe + ILP.
///
/// This strategy requires:
/// 1. Market dependency information (which markets are correlated)
/// 2. ILP constraints encoding those dependencies
/// 3. A solver to run the Frank-Wolfe algorithm
///
/// Without dependency information, this strategy does nothing.
pub struct CombinatorialStrategy {
    config: CombinatorialConfig,
    fw: FrankWolfe,
    /// Cluster cache for relation lookups.
    cluster_cache: Option<Arc<ClusterCache>>,
    /// Market registry for resolving market IDs.
    registry: Option<Arc<MarketRegistry>>,
    /// Cluster detector for running Frank-Wolfe detection.
    detector: Option<ClusterDetector>,
}

impl CombinatorialStrategy {
    /// Create a new strategy with the given configuration.
    #[must_use]
    pub fn new(config: CombinatorialConfig) -> Self {
        let fw_config = FrankWolfeConfig {
            max_iterations: config.max_iterations,
            tolerance: config.tolerance,
        };
        Self {
            config,
            fw: FrankWolfe::new(fw_config),
            cluster_cache: None,
            registry: None,
            detector: None,
        }
    }

    /// Set the cluster cache for relation lookups.
    pub fn set_cache(&mut self, cache: Arc<ClusterCache>) {
        self.cluster_cache = Some(cache);
        self.update_detector();
    }

    /// Set the market registry for market lookups.
    pub fn set_registry(&mut self, registry: Arc<MarketRegistry>) {
        self.registry = Some(registry);
    }

    /// Create strategy with cache already set.
    #[must_use]
    pub fn with_cache(mut self, cache: Arc<ClusterCache>) -> Self {
        self.cluster_cache = Some(cache);
        self.update_detector();
        self
    }

    /// Create strategy with registry already set.
    #[must_use]
    pub fn with_registry(mut self, registry: Arc<MarketRegistry>) -> Self {
        self.registry = Some(registry);
        self
    }

    /// Update the detector instance based on current config.
    fn update_detector(&mut self) {
        let detector_config = ClusterDetectionConfig {
            debounce_ms: 100, // Not used in synchronous detection
            min_gap: self.config.gap_threshold,
            max_clusters_per_cycle: 50, // Not relevant for single detection
        };
        self.detector = Some(ClusterDetector::new(detector_config));
    }

    /// Check if a market has known relations in the cache.
    fn has_cached_relations(&self, market_id: &crate::core::domain::MarketId) -> bool {
        self.cluster_cache
            .as_ref()
            .map(|c| c.has_relations(market_id))
            .unwrap_or(false)
    }

    /// Get the strategy configuration.
    #[must_use]
    pub const fn config(&self) -> &CombinatorialConfig {
        &self.config
    }

    /// Get the Frank-Wolfe algorithm instance.
    #[must_use]
    pub const fn frank_wolfe(&self) -> &FrankWolfe {
        &self.fw
    }
}

impl Strategy for CombinatorialStrategy {
    fn name(&self) -> &'static str {
        "combinatorial"
    }

    fn applies_to(&self, ctx: &MarketContext) -> bool {
        // Only applies to markets with known dependencies
        // Check both static context and dynamic cache
        if !self.config.enabled {
            return false;
        }

        // If context says it has dependencies, trust that
        if ctx.has_dependencies {
            return true;
        }

        // Otherwise check cache for any correlated markets
        ctx.correlated_markets
            .first()
            .map(|m| self.has_cached_relations(m))
            .unwrap_or(false)
    }

    fn detect(&self, ctx: &DetectionContext) -> Vec<Opportunity> {
        // Ensure we have all required components
        let cluster_cache = match &self.cluster_cache {
            Some(c) => c,
            None => return vec![],
        };

        let registry = match &self.registry {
            Some(r) => r,
            None => {
                tracing::warn!("Combinatorial strategy missing market registry");
                return vec![];
            }
        };

        let detector = match &self.detector {
            Some(d) => d,
            None => {
                tracing::warn!("Combinatorial strategy missing detector");
                return vec![];
            }
        };

        // Get cluster for this market
        let cluster = match cluster_cache.get_for_market(ctx.market.market_id()) {
            Some(c) => c,
            None => return vec![], // No known relations
        };

        tracing::debug!(
            market_id = %ctx.market.market_id(),
            cluster_size = cluster.markets.len(),
            constraint_count = cluster.constraints.len(),
            "Running combinatorial detection on cluster"
        );

        // Run cluster detection
        match detector.detect(&cluster, ctx.cache, registry) {
            Ok(Some(cluster_opp)) => {
                tracing::info!(
                    market_id = %ctx.market.market_id(),
                    gap = %cluster_opp.gap,
                    "Found combinatorial opportunity"
                );
                vec![cluster_opp.opportunity]
            }
            Ok(None) => {
                tracing::trace!(
                    market_id = %ctx.market.market_id(),
                    "No combinatorial opportunity (gap below threshold)"
                );
                vec![]
            }
            Err(e) => {
                tracing::debug!(
                    market_id = %ctx.market.market_id(),
                    error = %e,
                    "Combinatorial detection failed"
                );
                vec![]
            }
        }
    }

    fn set_market_registry(&mut self, registry: Arc<MarketRegistry>) {
        self.set_registry(registry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::cache::OrderBookCache;
    use crate::core::domain::{Cluster, ClusterId, Market, MarketId, Outcome, PriceLevel, TokenId};
    use crate::core::solver::Constraint;
    use chrono::{Duration, Utc};
    use rust_decimal_macros::dec;

    fn make_test_config() -> CombinatorialConfig {
        CombinatorialConfig {
            enabled: true,
            max_iterations: 20,
            tolerance: dec!(0.0001),
            gap_threshold: dec!(0.02),
        }
    }

    fn make_binary_market(id: &str, yes_token: &str, no_token: &str) -> Market {
        let outcomes = vec![
            Outcome::new(TokenId::from(yes_token), "Yes"),
            Outcome::new(TokenId::from(no_token), "No"),
        ];
        Market::new(
            MarketId::from(id),
            format!("Market {id}?"),
            outcomes,
            dec!(1),
        )
    }

    #[test]
    fn test_strategy_name() {
        let strategy = CombinatorialStrategy::new(CombinatorialConfig::default());
        assert_eq!(strategy.name(), "combinatorial");
    }

    #[test]
    fn test_disabled_by_default() {
        let config = CombinatorialConfig::default();
        assert!(!config.enabled);
    }

    #[test]
    fn test_applies_only_with_dependencies() {
        let config = CombinatorialConfig {
            enabled: true,
            ..Default::default()
        };

        let strategy = CombinatorialStrategy::new(config);

        // Should not apply to markets without dependencies
        assert!(!strategy.applies_to(&MarketContext::binary()));
        assert!(!strategy.applies_to(&MarketContext::multi_outcome(3)));

        // Should apply to markets with dependencies
        let ctx_with_deps = MarketContext::binary()
            .with_dependencies(vec![crate::core::domain::MarketId::from("other")]);
        assert!(strategy.applies_to(&ctx_with_deps));
    }

    #[test]
    fn test_config_defaults() {
        let config = CombinatorialConfig::default();

        assert_eq!(config.max_iterations, 20);
        assert_eq!(config.tolerance, Decimal::new(1, 4));
        assert_eq!(config.gap_threshold, Decimal::new(2, 2));
        assert!(!config.enabled);
    }

    #[test]
    fn test_strategy_requires_cluster_cache() {
        let config = make_test_config();
        let strategy = CombinatorialStrategy::new(config);

        let market = make_binary_market("m1", "yes", "no");
        let cache = OrderBookCache::new();
        let ctx = DetectionContext::new(&market, &cache);

        // Without cluster cache, should return empty
        let opps = strategy.detect(&ctx);
        assert!(opps.is_empty());
    }

    fn make_cluster(market_ids: Vec<MarketId>) -> Cluster {
        Cluster {
            id: ClusterId::new(),
            markets: market_ids,
            relations: vec![],
            constraints: vec![Constraint::geq(vec![dec!(1), dec!(1)], dec!(1))],
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_strategy_requires_registry() {
        let config = make_test_config();
        let mut strategy = CombinatorialStrategy::new(config);

        // Set cluster cache but not registry
        let cluster_cache = Arc::new(ClusterCache::new(Duration::hours(1)));
        strategy.set_cache(cluster_cache);

        let market = make_binary_market("m1", "yes", "no");
        let cache = OrderBookCache::new();
        let ctx = DetectionContext::new(&market, &cache);

        // Without registry, should return empty
        let opps = strategy.detect(&ctx);
        assert!(opps.is_empty());
    }

    #[test]
    fn test_detect_returns_empty_when_no_cluster() {
        let config = make_test_config();
        let mut strategy = CombinatorialStrategy::new(config);

        // Set both cache and registry
        let cluster_cache = Arc::new(ClusterCache::new(Duration::hours(1)));
        let registry = Arc::new(MarketRegistry::new());
        strategy.set_cache(cluster_cache);
        strategy.set_registry(registry);

        let market = make_binary_market("m1", "yes", "no");
        let cache = OrderBookCache::new();
        let ctx = DetectionContext::new(&market, &cache);

        // No cluster exists for this market
        let opps = strategy.detect(&ctx);
        assert!(opps.is_empty());
    }

    #[test]
    fn test_detect_returns_empty_when_gap_below_threshold() {
        let config = CombinatorialConfig {
            gap_threshold: dec!(0.10), // High threshold
            ..make_test_config()
        };
        let mut strategy = CombinatorialStrategy::new(config);

        // Create markets
        let m1 = make_binary_market("m1", "yes1", "no1");
        let m2 = make_binary_market("m2", "yes2", "no2");

        // Build registry
        let mut registry = MarketRegistry::new();
        registry.add(m1.clone());
        registry.add(m2.clone());
        let registry = Arc::new(registry);

        // Create cluster
        let cluster = make_cluster(vec![m1.market_id().clone(), m2.market_id().clone()]);

        let cluster_cache = Arc::new(ClusterCache::new(Duration::hours(1)));
        cluster_cache.put(cluster);

        strategy.set_cache(cluster_cache);
        strategy.set_registry(registry);

        // Set prices that are fair (no arbitrage)
        let cache = OrderBookCache::new();
        cache.update(crate::core::domain::OrderBook::with_levels(
            TokenId::from("yes1"),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(100))],
        ));
        cache.update(crate::core::domain::OrderBook::with_levels(
            TokenId::from("yes2"),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(100))],
        ));

        let ctx = DetectionContext::new(&m1, &cache);

        // Should return empty due to low/no arbitrage gap
        let opps = strategy.detect(&ctx);
        assert!(opps.is_empty());
    }

    #[test]
    fn test_detect_handles_missing_price_data() {
        let config = make_test_config();
        let mut strategy = CombinatorialStrategy::new(config);

        // Create markets
        let m1 = make_binary_market("m1", "yes1", "no1");
        let m2 = make_binary_market("m2", "yes2", "no2");

        // Build registry
        let mut registry = MarketRegistry::new();
        registry.add(m1.clone());
        registry.add(m2.clone());
        let registry = Arc::new(registry);

        // Create cluster
        let cluster = make_cluster(vec![m1.market_id().clone(), m2.market_id().clone()]);

        let cluster_cache = Arc::new(ClusterCache::new(Duration::hours(1)));
        cluster_cache.put(cluster);

        strategy.set_cache(cluster_cache);
        strategy.set_registry(registry);

        // Empty cache (no price data)
        let cache = OrderBookCache::new();
        let ctx = DetectionContext::new(&m1, &cache);

        // Should fail closed and return empty
        let opps = strategy.detect(&ctx);
        assert!(opps.is_empty());
    }

    #[test]
    fn test_strategy_with_cache_builder() {
        let config = make_test_config();
        let cluster_cache = Arc::new(ClusterCache::new(Duration::hours(1)));
        let strategy = CombinatorialStrategy::new(config).with_cache(cluster_cache);

        assert!(strategy.cluster_cache.is_some());
    }

    #[test]
    fn test_strategy_with_registry_builder() {
        let config = make_test_config();
        let registry = Arc::new(MarketRegistry::new());
        let strategy = CombinatorialStrategy::new(config).with_registry(registry);

        assert!(strategy.registry.is_some());
    }

    #[test]
    fn test_applies_to_checks_enabled_flag() {
        let config = CombinatorialConfig {
            enabled: false,
            ..Default::default()
        };
        let strategy = CombinatorialStrategy::new(config);

        let ctx_with_deps =
            MarketContext::binary().with_dependencies(vec![MarketId::from("other")]);

        // Even with dependencies, disabled strategy doesn't apply
        assert!(!strategy.applies_to(&ctx_with_deps));
    }

    #[test]
    fn test_applies_to_checks_cache_for_relations() {
        let config = make_test_config();
        let mut strategy = CombinatorialStrategy::new(config);

        let m1 = make_binary_market("m1", "yes1", "no1");
        let m2 = make_binary_market("m2", "yes2", "no2");

        // Create cluster
        let cluster = make_cluster(vec![m1.market_id().clone(), m2.market_id().clone()]);

        let cluster_cache = Arc::new(ClusterCache::new(Duration::hours(1)));
        cluster_cache.put(cluster);
        strategy.set_cache(cluster_cache);

        // Context without dependencies but market is in cache
        let ctx = MarketContext {
            outcome_count: 2,
            has_dependencies: false,
            correlated_markets: vec![m1.market_id().clone()],
        };

        // Should apply because cache has relations
        assert!(strategy.applies_to(&ctx));
    }
}
