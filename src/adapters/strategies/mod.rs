//! Strategy implementations for arbitrage detection.
//!
//! This module contains concrete strategy implementations that detect
//! different types of arbitrage opportunities.

pub mod combinatorial;
pub mod condition;
mod context;
pub mod rebalancing;
mod registry;

pub use combinatorial::{CombinatorialConfig, CombinatorialStrategy};
pub use condition::{SingleConditionConfig, SingleConditionStrategy};
pub use context::{DetectionContext, DetectionResult, MarketContext};
pub use rebalancing::{
    MarketRebalancingConfig, MarketRebalancingStrategy, RebalancingLeg, RebalancingOpportunity,
};
pub use registry::{StrategyRegistry, StrategyRegistryBuilder};
