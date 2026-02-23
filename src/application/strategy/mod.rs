//! Strategy implementations for arbitrage detection.
//!
//! Provides concrete detection algorithms implementing the
//! [`Strategy`](crate::port::inbound::strategy::Strategy) trait:
//!
//! - [`single_condition`]: Binary market arbitrage (YES + NO < $1)
//! - [`market_rebalancing`]: Multi-outcome arbitrage (sum of all outcomes < $1)
//! - [`combinatorial`]: Cross-market arbitrage using Frank-Wolfe projection
//!
//! Use [`registry::StrategyRegistry`] to manage and run multiple strategies.

pub mod combinatorial;
pub mod context;
pub mod market_rebalancing;
pub mod registry;
pub mod single_condition;
