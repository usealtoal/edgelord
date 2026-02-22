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
//! - Add strategies: implement `ports::Strategy`
//! - Add exchanges: implement `ports::MarketDataStream` + `ports::ArbitrageExecutor`
//! - Add notifiers: implement `ports::Notifier`
//!
//! # Architecture
//!
//! ```text
//! domain/     Pure types, no I/O
//! ports/      Trait definitions (extension points)
//! adapter/    Implementations (Polymarket, strategies, etc.)
//! runtime/    Orchestration and wiring
//! cli/        Command-line interface
//! ```

pub mod adapter;
pub mod cli;
pub mod domain;
pub mod error;
pub mod ports;
pub mod runtime;

#[cfg(any(test, feature = "testkit"))]
pub mod testkit;
