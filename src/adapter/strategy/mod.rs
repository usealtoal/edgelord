//! Strategy implementations for arbitrage detection.
//!
//! This module contains concrete strategy implementations that detect
//! different types of arbitrage opportunities.

use std::sync::Arc;

use crate::domain::{MarketRegistry, Opportunity};

pub mod combinatorial;
pub mod condition;
mod context;
pub mod rebalancing;
mod registry;

// Re-export types from port
pub use crate::port::{DetectionResult, MarketContext};
pub use combinatorial::{CombinatorialConfig, CombinatorialStrategy};
pub use condition::{SingleConditionConfig, SingleConditionStrategy};
pub use context::DetectionContext;
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
    fn name(&self) -> &'static str;

    /// Check if this strategy should run for a given market context.
    fn applies_to(&self, ctx: &MarketContext) -> bool;

    /// Detect opportunities given current market state.
    fn detect(&self, ctx: &DetectionContext) -> Vec<Opportunity>;

    /// Optional: provide warm-start hint from previous detection.
    fn warm_start(&mut self, _previous: &DetectionResult) {}

    /// Optional: inject the market registry for strategies that need it.
    fn set_market_registry(&mut self, _registry: Arc<MarketRegistry>) {}
}
