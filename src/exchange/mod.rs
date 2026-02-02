//! Exchange abstraction layer.
//!
//! Defines traits that exchange implementations must fulfill,
//! enabling multi-exchange support with a common interface.

mod factory;
mod traits;

pub use factory::ExchangeFactory;
pub use traits::{
    ExecutionResult, MarketDataStream, MarketEvent, MarketFetcher, MarketInfo, OrderExecutor,
    OrderId, OrderRequest, OrderSide, OutcomeInfo,
};
