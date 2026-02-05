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


pub use bregman::{bregman_divergence, bregman_gradient, lmsr_cost, lmsr_prices};
pub use frank_wolfe::{FrankWolfe, FrankWolfeConfig, FrankWolfeResult};

use std::sync::Arc;

use rust_decimal::Decimal;
use serde::Deserialize;

use super::{DetectionContext, MarketContext, Strategy};
use crate::core::cache::ClusterCache;
use crate::core::domain::Opportunity;

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
        }
    }

    /// Set the cluster cache for relation lookups.
    pub fn set_cache(&mut self, cache: Arc<ClusterCache>) {
        self.cluster_cache = Some(cache);
    }

    /// Create strategy with cache already set.
    #[must_use]
    pub fn with_cache(mut self, cache: Arc<ClusterCache>) -> Self {
        self.cluster_cache = Some(cache);
        self
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
        // Get cluster from cache
        let cache = match &self.cluster_cache {
            Some(c) => c,
            None => return vec![],
        };

        let cluster = match cache.get_for_market(ctx.market.market_id()) {
            Some(c) => c,
            None => return vec![], // No known relations
        };

        // Log that we found a cluster (actual Frank-Wolfe execution is complex)
        tracing::debug!(
            market_id = %ctx.market.market_id(),
            cluster_size = cluster.markets.len(),
            constraint_count = cluster.constraints.len(),
            "Found cluster for combinatorial detection"
        );

        // Full Frank-Wolfe execution requires:
        // 1. Gather prices for all markets in cluster
        // 2. Build ILP problem from cluster.constraints
        // 3. Run self.fw.project()
        // 4. Check if gap > threshold
        // 5. Build opportunity with multiple legs
        //
        // This is the integration point - for now return empty
        // Real implementation needs multi-market price aggregation
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
