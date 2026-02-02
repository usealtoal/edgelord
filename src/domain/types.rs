//! Core domain types for arbitrage detection.
//!
//! NOTE: This file is being deprecated. Types are being migrated to focused modules:
//! - TokenId, MarketId -> ids.rs
//! - Price, Volume -> money.rs
//! - MarketPair, MarketInfo, TokenInfo -> market.rs
//!
//! This file will be deleted in Task 14. For now, it contains:
//! - OrderBook and PriceLevel (will move to orderbook types)
//! - Opportunity (will move to its own module)

use super::ids::{MarketId, TokenId};
use super::money::{Price, Volume};

/// A single price level in the order book
#[derive(Debug, Clone)]
pub struct PriceLevel {
    pub price: Price,
    pub size: Volume,
}

/// Order book for a single token
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct OrderBook {
    pub token_id: TokenId,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
}

impl OrderBook {
    #[allow(dead_code)]
    pub fn new(token_id: TokenId) -> Self {
        Self {
            token_id,
            bids: Vec::new(),
            asks: Vec::new(),
        }
    }

    /// Best bid (highest buy price)
    #[allow(dead_code)]
    pub fn best_bid(&self) -> Option<&PriceLevel> {
        self.bids.first()
    }

    /// Best ask (lowest sell price)
    pub fn best_ask(&self) -> Option<&PriceLevel> {
        self.asks.first()
    }
}

/// Detected arbitrage opportunity
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Opportunity {
    pub market_id: MarketId,
    pub question: String,
    pub yes_token: TokenId,
    pub no_token: TokenId,
    pub yes_ask: Price,
    pub no_ask: Price,
    pub total_cost: Price,
    pub edge: Price,
    pub volume: Volume,
    pub expected_profit: Price,
}
