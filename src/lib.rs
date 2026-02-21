//! Edgelord - Multi-strategy arbitrage detection and execution.
//!
//! # Architecture
//!
//! ```text
//! src/
//! ├── core/             # Reusable library components
//! │   ├── domain/       # Pure domain types
//! │   ├── exchange/     # Exchange traits + implementations
//! │   ├── strategy/     # Detection algorithms
//! │   ├── solver/       # LP/ILP solver abstraction
//! │   └── service/      # Cross-cutting services
//! └── app/              # Application orchestration
//! ```
//!
//! # Features
//!
//! - `polymarket` - Enable Polymarket exchange support (default)
//! - `telegram` - Enable Telegram notifications
//! - `testkit` - Reusable test helpers (mock streams, factories, domain builders)

pub mod app;
pub mod cli;
pub mod core;
pub mod domain;
pub mod error;

#[cfg(any(test, feature = "testkit"))]
pub mod testkit;
