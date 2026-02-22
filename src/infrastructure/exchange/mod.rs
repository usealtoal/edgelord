//! Exchange abstraction layer.
//!
//! Provides connection management and runtime exchange selection.
//!
//! ## Adding a New Exchange
//!
//! 1. Create a module under `adapter/outbound/<exchange>/`
//! 2. Implement [`crate::port::outbound::exchange::MarketDataStream`] and [`crate::port::outbound::exchange::ArbitrageExecutor`]
//! 3. Implement [`crate::port::outbound::exchange::MarketParser`] trait
//! 4. Add to [`ExchangeFactory`] for runtime selection

pub mod factory;
pub mod pool;
pub mod reconnecting;
