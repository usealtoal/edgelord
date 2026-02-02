//! Core domain types for arbitrage detection.

use rust_decimal::Decimal;
use std::fmt;

/// Token identifier - newtype for type safety
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TokenId(pub String);

impl fmt::Display for TokenId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for TokenId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for TokenId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Market condition identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MarketId(pub String);

impl fmt::Display for MarketId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for MarketId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Price and volume use Decimal for precision
pub type Price = Decimal;
pub type Volume = Decimal;

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

/// A YES/NO market pair
#[derive(Debug, Clone)]
pub struct MarketPair {
    pub market_id: MarketId,
    pub question: String,
    pub yes_token: TokenId,
    pub no_token: TokenId,
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
