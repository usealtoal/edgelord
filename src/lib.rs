//! Edgelord - Multi-strategy arbitrage detection and execution.
//!
//! # Architecture
//!
//! ```text
//! src/
//! ├── domain/          # Exchange-agnostic business logic
//! │   ├── strategy/    # Pluggable detection strategies
//! │   └── solver/      # LP/ILP solver abstraction
//! ├── exchange/        # Exchange trait definitions
//! ├── adapter/         # Exchange implementations
//! │   └── polymarket/  # Polymarket CLOB integration
//! ├── service/         # Cross-cutting services (risk, notifications)
//! └── app/             # Application layer (config, state, orchestration)
//! ```
//!
//! # Features
//!
//! - `polymarket` - Enable Polymarket exchange support (default)
//! - `telegram` - Enable Telegram notifications (coming soon)

pub mod domain;
pub mod error;
pub mod exchange;
pub mod adapter;
pub mod service;
pub mod app;
pub mod cli;
