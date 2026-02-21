//! Polymarket exchange integration.
//!
//! This module re-exports from `crate::adapters::polymarket` for backward compatibility.

pub use crate::adapters::polymarket::{
    PolymarketApproval, PolymarketBookMessage, PolymarketClient, PolymarketDataStream,
    PolymarketDeduplicator, PolymarketExchangeConfig, PolymarketExecutor, PolymarketFilter,
    PolymarketMarket, PolymarketScorer, PolymarketTaggedMessage, PolymarketToken,
    PolymarketWebSocketHandler, PolymarketWsMessage, PolymarketWsPriceLevel, SweepResult,
    POLYMARKET_PAYOUT,
};
