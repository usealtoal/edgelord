//! Order book types.

use super::id::TokenId;
use super::money::{Price, Volume};

/// A single price level in the book.
#[derive(Debug, Clone)]
pub struct PriceLevel {
    price: Price,
    size: Volume,
}

impl PriceLevel {
    /// Create a new price level.
    #[must_use]
    pub const fn new(price: Price, size: Volume) -> Self {
        Self { price, size }
    }

    /// Get the price.
    #[must_use]
    pub const fn price(&self) -> Price {
        self.price
    }

    /// Get the size/volume.
    #[must_use]
    pub const fn size(&self) -> Volume {
        self.size
    }
}

/// Order book for a single token.
#[derive(Debug, Clone)]
pub struct Book {
    token_id: TokenId,
    bids: Vec<PriceLevel>,
    asks: Vec<PriceLevel>,
}

impl Book {
    /// Create a new empty book.
    #[must_use]
    pub const fn new(token_id: TokenId) -> Self {
        Self {
            token_id,
            bids: Vec::new(),
            asks: Vec::new(),
        }
    }

    /// Create a book with initial levels.
    #[must_use]
    pub const fn with_levels(
        token_id: TokenId,
        bids: Vec<PriceLevel>,
        asks: Vec<PriceLevel>,
    ) -> Self {
        Self {
            token_id,
            bids,
            asks,
        }
    }

    /// Get the token ID.
    #[must_use]
    pub const fn token_id(&self) -> &TokenId {
        &self.token_id
    }

    /// Get all bid levels.
    #[must_use]
    pub fn bids(&self) -> &[PriceLevel] {
        &self.bids
    }

    /// Get all ask levels.
    #[must_use]
    pub fn asks(&self) -> &[PriceLevel] {
        &self.asks
    }

    /// Best bid (highest buy price).
    #[must_use]
    pub fn best_bid(&self) -> Option<&PriceLevel> {
        self.bids.first()
    }

    /// Best ask (lowest sell price).
    #[must_use]
    pub fn best_ask(&self) -> Option<&PriceLevel> {
        self.asks.first()
    }
}
