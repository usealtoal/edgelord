//! Thread-safe order book cache.

use parking_lot::RwLock;
use std::collections::HashMap;

use super::ids::TokenId;
use super::types::OrderBook;

/// Thread-safe cache of order books
pub struct OrderBookCache {
    books: RwLock<HashMap<TokenId, OrderBook>>,
}

impl OrderBookCache {
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.books.read().len()
    }

    #[allow(dead_code)]
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
    use crate::domain::PriceLevel;
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
