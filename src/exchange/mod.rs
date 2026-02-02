//! Exchange abstraction layer.
//!
//! Defines traits that exchange implementations must fulfill,
//! enabling multi-exchange support with a common interface.

mod traits;

pub use traits::{
    ExchangeClient, ExecutionResult, MarketInfo, OrderExecutor, OrderId, OrderRequest, OrderSide,
};
