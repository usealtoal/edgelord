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
    price: Price,
    size: Volume,
}

impl PriceLevel {
    /// Create a new price level
    pub fn new(price: Price, size: Volume) -> Self {
        Self { price, size }
    }

    /// Get the price
    pub fn price(&self) -> Price {
        self.price
    }

    /// Get the size/volume
    pub fn size(&self) -> Volume {
        self.size
    }
}

/// Order book for a single token
#[derive(Debug, Clone)]
pub struct OrderBook {
    token_id: TokenId,
    bids: Vec<PriceLevel>,
    asks: Vec<PriceLevel>,
}

impl OrderBook {
    /// Create a new empty order book
    pub fn new(token_id: TokenId) -> Self {
        Self {
            token_id,
            bids: Vec::new(),
            asks: Vec::new(),
        }
    }

    /// Create an order book with initial levels
    pub fn with_levels(token_id: TokenId, bids: Vec<PriceLevel>, asks: Vec<PriceLevel>) -> Self {
        Self {
            token_id,
            bids,
            asks,
        }
    }

    /// Get the token ID
    pub fn token_id(&self) -> &TokenId {
        &self.token_id
    }

    /// Get all bid levels
    pub fn bids(&self) -> &[PriceLevel] {
        &self.bids
    }

    /// Get all ask levels
    pub fn asks(&self) -> &[PriceLevel] {
        &self.asks
    }

    /// Best bid (highest buy price)
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
