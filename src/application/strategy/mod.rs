//! Strategy implementations for arbitrage detection.
//!
//! Implements `port::inbound::strategy::Strategy` with concrete detection algorithms.

pub mod combinatorial;
pub mod context;
pub mod market_rebalancing;
pub mod registry;
pub mod single_condition;
