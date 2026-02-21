//! Shared test utilities available to both unit and integration tests.
//!
//! Enabled via `#[cfg(test)]` (unit tests) or the `testkit` feature
//! (integration tests).
//!
//! # Modules
//!
//! - [`stream`] — Mock [`MarketDataStream`](crate::runtime::exchange::MarketDataStream)
//!   implementations: `ScriptedStream`, `CyclingStream`, `ChannelStream`.
//! - [`domain`] — Builders for domain primitives: tokens, markets, events.
//! - [`config`] — Canonical test configurations (reconnection, pool, etc.).

pub mod config;
pub mod domain;
pub mod stream;
