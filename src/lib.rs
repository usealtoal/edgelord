//! Edgelord - Prediction market arbitrage framework.
//!
//! # For CLI users
//!
//! Install and run:
//!
//! ```text
//! cargo install edgelord
//! edgelord init
//! edgelord run
//! ```
//!
//! # For developers
//!
//! Fork this repo and extend:
//!
//! - Add strategies: implement `port::Strategy`
//! - Add exchanges: implement `port::MarketDataStream` + `port::ArbitrageExecutor`
//! - Add notifiers: implement `port::Notifier`
//!
//! # Architecture
//!
//! ```text
//! domain/          Pure types, no I/O
//! port/            Trait definitions (extension points)
//! adapter/         Port implementations (external integrations)
//! application/     Application services (use cases)
//! infrastructure/  Runtime concerns (caching, config, orchestration)
//! cli/             Command-line interface
//! ```

pub mod adapter;
pub mod application;
pub mod cli;
pub mod domain;
pub mod error;
pub mod infrastructure;
pub mod port;

#[cfg(any(test, feature = "testkit"))]
pub mod testkit;
