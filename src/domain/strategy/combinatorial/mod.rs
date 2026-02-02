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

mod bregman;
mod frank_wolfe;

pub use bregman::{bregman_divergence, bregman_gradient, lmsr_cost, lmsr_prices};
pub use frank_wolfe::{FrankWolfe, FrankWolfeConfig, FrankWolfeResult};

use rust_decimal::Decimal;
use serde::Deserialize;

use super::{DetectionContext, MarketContext, Strategy};
use crate::domain::Opportunity;

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

fn default_max_iterations() -> usize {
    20
}

fn default_tolerance() -> Decimal {
    Decimal::new(1, 4) // 0.0001
}

fn default_gap_threshold() -> Decimal {
    Decimal::new(2, 2) // 0.02
}

fn default_enabled() -> bool {
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
        }
    }

    /// Get the strategy configuration.
    #[must_use] 
    pub fn config(&self) -> &CombinatorialConfig {
        &self.config
    }

    /// Get the Frank-Wolfe algorithm instance.
    #[must_use] 
    pub fn frank_wolfe(&self) -> &FrankWolfe {
        &self.fw
    }
}

impl Strategy for CombinatorialStrategy {
    fn name(&self) -> &'static str {
        "combinatorial"
    }

    fn applies_to(&self, ctx: &MarketContext) -> bool {
        // Only applies to markets with known dependencies
        self.config.enabled && ctx.has_dependencies
    }

    fn detect(&self, _ctx: &DetectionContext) -> Vec<Opportunity> {
        // Full implementation requires:
        // 1. Market dependency graph (which markets are correlated)
        // 2. ILP constraint builder (encode dependencies as constraints)
        // 3. Multi-market state aggregation (prices across correlated markets)
        //
        // This is a complex feature requiring:
        // - Dependency detection (potentially LLM-assisted as in the research)
        // - Constraint encoding for various dependency types
        // - Efficient state management across market clusters
        //
        // For now, return empty. Real implementation would:
        // 1. Get correlated markets from context
        // 2. Build ILP from dependency constraints
        // 3. Run Frank-Wolfe projection
        // 4. If gap > threshold, create opportunity
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
        let mut config = CombinatorialConfig::default();
        config.enabled = true;

        let strategy = CombinatorialStrategy::new(config);

        // Should not apply to markets without dependencies
        assert!(!strategy.applies_to(&MarketContext::binary()));
        assert!(!strategy.applies_to(&MarketContext::multi_outcome(3)));

        // Should apply to markets with dependencies
        let ctx_with_deps = MarketContext::binary()
            .with_dependencies(vec![crate::domain::MarketId::from("other")]);
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
