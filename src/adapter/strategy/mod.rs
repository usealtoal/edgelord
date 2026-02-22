//! Strategy implementations for arbitrage detection.
//!
//! Implements the `port::Strategy` trait with concrete detection algorithms.

pub mod combinatorial;
pub mod condition;
mod context;
pub mod rebalancing;
mod registry;

pub use combinatorial::{CombinatorialConfig, CombinatorialStrategy};
pub use condition::{SingleConditionConfig, SingleConditionStrategy};
pub use context::ConcreteDetectionContext;
pub use rebalancing::{
    MarketRebalancingConfig, MarketRebalancingStrategy, RebalancingLeg, RebalancingOpportunity,
};
pub use registry::{StrategyRegistry, StrategyRegistryBuilder};
