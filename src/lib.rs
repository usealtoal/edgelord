//! Edgelord - Polymarket arbitrage detection and execution.
//!
//! # Architecture
//!
//! - `domain` - Exchange-agnostic types and logic
//! - `exchange` - Trait definitions for exchange implementations
//! - `polymarket` - Polymarket-specific implementation
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
pub mod polymarket;

pub mod app;
