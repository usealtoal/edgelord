//! Edgelord - Multi-strategy arbitrage detection and execution.
//!
//! This crate provides tools for detecting and executing arbitrage opportunities
//! on prediction markets using pluggable detection strategies.
//!
//! # Architecture
//!
//! The crate uses a strategy pattern for modular arbitrage detection:
//!
//! - **`domain::strategy`** - Pluggable detection strategies
//!   - `SingleConditionStrategy` - YES + NO < $1 (26.7% of historical profits)
//!   - `MarketRebalancingStrategy` - Sum of outcomes < $1 (73.1% of profits)
//!   - `CombinatorialStrategy` - Frank-Wolfe + ILP for correlated markets (0.24%)
//!
//! - **`domain::solver`** - LP/ILP solver abstraction
//!   - `HiGHSSolver` - Open-source HiGHS via good_lp
//!
//! - **`exchange`** - Exchange abstraction layer
//! - **`polymarket`** - Polymarket implementation (requires `polymarket` feature)
//!
//! # Modules
//!
//! - [`config`] - Configuration loading from TOML files with strategy settings
//! - [`domain`] - Exchange-agnostic types: order books, opportunities, positions
//! - [`domain::strategy`] - Strategy trait and implementations
//! - [`domain::solver`] - LP/ILP solver abstraction
//! - [`error`] - Error types for the crate
//! - [`exchange`] - Trait definitions for exchange implementations
//! - [`polymarket`] - Polymarket-specific implementation (requires `polymarket` feature)
//! - [`app`] - Application orchestration (requires `polymarket` feature)
//!
//! # Features
//!
//! - `polymarket` - Enable Polymarket exchange support (WebSocket, REST API, execution)
//!
//! # Example
//!
//! ```no_run
//! use edgelord::config::Config;
//! use edgelord::domain::strategy::{SingleConditionStrategy, StrategyRegistry};
//!
//! let mut registry = StrategyRegistry::new();
//! registry.register(Box::new(SingleConditionStrategy::new(Default::default())));
//! ```

pub mod config;
pub mod domain;
pub mod error;
pub mod exchange;

#[cfg(feature = "polymarket")]
pub mod adapter;

#[cfg(feature = "polymarket")]
pub mod app;
