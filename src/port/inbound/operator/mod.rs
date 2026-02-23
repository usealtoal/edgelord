//! Operator-facing inbound ports for CLI and administrative interfaces.
//!
//! Defines use-case interfaces consumed by operator-facing adapters such as
//! the CLI, Telegram bot, and administrative tools.
//!
//! # Modules
//!
//! - [`config`]: Configuration display and validation
//! - [`diagnostic`]: Health checks and connectivity diagnostics
//! - [`port`]: Unified operator capability surface
//! - [`runtime`]: Runtime control and monitoring
//! - [`stats`]: Trading statistics and reporting
//! - [`status`]: Current status snapshots
//! - [`wallet`]: Wallet management and approvals

pub mod config;
pub mod diagnostic;
pub mod port;
pub mod runtime;
pub mod stats;
pub mod status;
pub mod wallet;
