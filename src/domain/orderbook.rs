//! Order book types and thread-safe cache.

use parking_lot::RwLock;
use std::collections::HashMap;

use super::id::TokenId;
use super::money::{Price, Volume};

/// A single price level in the order book
#[derive(Debug, Clone)]
pub struct PriceLevel {
    price: Price,
    size: Volume,
}

impl PriceLevel {
    /// Create a new price level
    #[must_use] 
    pub fn new(price: Price, size: Volume) -> Self {
        Self { price, size }
    }

    /// Get the price
    #[must_use] 
    pub fn price(&self) -> Price {
        self.price
    }

    /// Get the size/volume
    #[must_use] 
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
    #[must_use] 
    pub fn new(token_id: TokenId) -> Self {
        Self {
            token_id,
            bids: Vec::new(),
            asks: Vec::new(),
        }
    }

    /// Create an order book with initial levels
    #[must_use] 
    pub fn with_levels(token_id: TokenId, bids: Vec<PriceLevel>, asks: Vec<PriceLevel>) -> Self {
        Self {
            token_id,
            bids,
            asks,
        }
    }

    /// Get the token ID
    #[must_use] 
    pub fn token_id(&self) -> &TokenId {
        &self.token_id
    }

    /// Get all bid levels
    #[must_use] 
    pub fn bids(&self) -> &[PriceLevel] {
        &self.bids
    }

    /// Get all ask levels
    #[must_use] 
    pub fn asks(&self) -> &[PriceLevel] {
        &self.asks
    }

    /// Best bid (highest buy price)
    #[must_use] 
    pub fn best_bid(&self) -> Option<&PriceLevel> {
        self.bids.first()
    }

    /// Best ask (lowest sell price)
    #[must_use] 
    pub fn best_ask(&self) -> Option<&PriceLevel> {
        self.asks.first()
    }
}

/// Thread-safe cache of order books
pub struct OrderBookCache {
    books: RwLock<HashMap<TokenId, OrderBook>>,
}

impl OrderBookCache {
    #[must_use] 
    pub fn new() -> Self {
        Self {
            books: RwLock::new(HashMap::new()),
        }
    }

    /// Update order book in the cache
    pub fn update(&self, book: OrderBook) {
        let token_id = book.token_id().clone();
        self.books.write().insert(token_id, book);
    }

    /// Get a snapshot of an order book
    pub fn get(&self, token_id: &TokenId) -> Option<OrderBook> {
        self.books.read().get(token_id).cloned()
    }

    /// Get snapshots of two order books atomically
    pub fn get_pair(
        &self,
        token_a: &TokenId,
        token_b: &TokenId,
    ) -> (Option<OrderBook>, Option<OrderBook>) {
        let books = self.books.read();
        (books.get(token_a).cloned(), books.get(token_b).cloned())
    }

    /// Number of books in cache
    pub fn len(&self) -> usize {
        self.books.read().len()
    }

    /// Returns true if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for OrderBookCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::PriceLevel;
    use rust_decimal_macros::dec;

    #[test]
    fn test_update_and_get() {
        let cache = OrderBookCache::new();
        let token_id = TokenId::from("test-token");

        let book = OrderBook::with_levels(
            token_id.clone(),
            vec![PriceLevel::new(dec!(0.45), dec!(100))],
            vec![PriceLevel::new(dec!(0.50), dec!(100))],
        );

        cache.update(book);

        let retrieved = cache.get(&token_id);
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.best_bid().unwrap().price(), dec!(0.45));
        assert_eq!(retrieved.best_ask().unwrap().price(), dec!(0.50));
    }

    #[test]
    fn test_get_pair() {
        let cache = OrderBookCache::new();
        let token_a = TokenId::from("token-a");
        let token_b = TokenId::from("token-b");

        let book_a = OrderBook::with_levels(
            token_a.clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.40), dec!(50))],
        );
        let book_b = OrderBook::with_levels(
            token_b.clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.55), dec!(75))],
        );

        cache.update(book_a);
        cache.update(book_b);

        let (a, b) = cache.get_pair(&token_a, &token_b);
        assert!(a.is_some());
        assert!(b.is_some());

        assert_eq!(a.unwrap().best_ask().unwrap().price(), dec!(0.40));
        assert_eq!(b.unwrap().best_ask().unwrap().price(), dec!(0.55));
    }
}
