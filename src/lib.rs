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

pub mod app;
pub mod cli;
pub mod core;
pub mod error;
