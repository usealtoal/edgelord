//! Exchange abstraction layer.
//!
//! DEPRECATED: This module is being phased out. Use `crate::runtime::exchange` instead.

// Keep polymarket submodule (it's still in adapters)
pub mod polymarket;

// Re-export from new location for backward compatibility
pub use crate::domain::PoolStats;
pub use crate::runtime::exchange::{
    ApprovalResult, ApprovalStatus, ArbitrageExecutor, ConnectionPool, DedupConfig, DedupStrategy,
    ExchangeConfig, ExchangeFactory, ExecutionResult, MarketDataStream, MarketEvent, MarketFetcher,
    MarketFilter, MarketFilterConfig, MarketInfo, MarketScorer, MessageDeduplicator, OrderExecutor,
    OrderRequest, OrderSide, OutcomeInfo, ReconnectingDataStream, StreamFactory, TokenApproval,
};
