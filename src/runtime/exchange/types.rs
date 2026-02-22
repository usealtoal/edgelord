//! Exchange types re-exported from port.
//!
//! This module re-exports types from the port module to maintain backward
//! compatibility while centralizing trait definitions.

pub use crate::port::{
    ArbitrageExecutor, ExecutionResult, MarketDataStream, MarketEvent, MarketFetcher, MarketInfo,
    OrderExecutor, OrderRequest, OrderSide, OutcomeInfo,
};
