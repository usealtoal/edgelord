//! Telegram notification and command handling.
//!
//! Provides Telegram bot integration for real-time trade notifications and
//! interactive bot commands for controlling the arbitrage detector remotely.

mod auth;
mod command;
mod format;

pub mod control;
pub mod notifier;
