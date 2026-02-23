//! Inbound (driving) ports consumed by inbound adapters.
//!
//! Inbound ports expose application capabilities to external drivers such as:
//!
//! - Command-line interface (CLI)
//! - Telegram bot control surface
//! - Runtime orchestration entry points
//!
//! # Modules
//!
//! - [`operator`]: Operator-facing use cases for configuration, diagnostics, and wallet management
//! - [`risk`]: Risk check result types for trade validation
//! - [`runtime`]: Runtime state and control interfaces
//! - [`strategy`]: Arbitrage detection strategy interfaces

pub mod operator;
pub mod risk;
pub mod runtime;
pub mod strategy;
