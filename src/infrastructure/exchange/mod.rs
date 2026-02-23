//! Exchange abstraction layer.
//!
//! Provides connection management, pooling, and runtime exchange selection.
//! This module contains exchange-agnostic infrastructure that works with
//! any exchange implementation.
//!
//! # Submodules
//!
//! - [`factory`] - Exchange component factory for runtime selection
//! - [`pool`] - WebSocket connection pool for subscription distribution
//! - [`reconnecting`] - Auto-reconnecting stream wrapper
//!
//! # Adding a New Exchange
//!
//! 1. Create a module under `adapter/outbound/<exchange>/`
//! 2. Implement [`crate::port::outbound::exchange::MarketDataStream`] and
//!    [`crate::port::outbound::exchange::ArbitrageExecutor`]
//! 3. Implement [`crate::port::outbound::exchange::MarketParser`] trait
//! 4. Add to [`factory::ExchangeFactory`] for runtime selection

pub mod factory;
pub mod pool;
pub mod reconnecting;
