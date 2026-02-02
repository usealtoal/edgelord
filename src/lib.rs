//! Edgelord - Polymarket arbitrage detection and execution.
//!
//! This crate provides tools for detecting and executing arbitrage opportunities
//! on prediction markets, specifically binary YES/NO markets where the combined
//! price of both outcomes should equal $1.00.
//!
//! # Architecture
//!
//! The crate is organized into exchange-agnostic core logic and exchange-specific
//! implementations:
//!
//! - [`config`] - Configuration loading from TOML files
//! - [`domain`] - Exchange-agnostic types: order books, opportunities, positions
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
//! use edgelord::domain::DetectorConfig;
//!
//! let config = Config::load("config.toml").unwrap();
//! ```

pub mod config;
pub mod domain;
pub mod error;
pub mod exchange;

#[cfg(feature = "polymarket")]
pub mod polymarket;

#[cfg(feature = "polymarket")]
pub mod app;
