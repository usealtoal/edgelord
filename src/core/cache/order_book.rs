//! Thread-safe order book cache with optional update notifications.

use parking_lot::RwLock;
use std::collections::HashMap;
use tokio::sync::broadcast;

use crate::core::domain::{OrderBook, TokenId};

/// Notification sent when an order book is updated.
#[derive(Debug, Clone)]
pub struct OrderBookUpdate {
    /// The token that was updated.
    pub token_id: TokenId,
}

/// Thread-safe cache of order books with optional broadcast notifications.
pub struct OrderBookCache {
    books: RwLock<HashMap<TokenId, OrderBook>>,
    /// Broadcast sender for update notifications.
    /// Wrapped in Option to allow construction without notifications.
    tx: Option<broadcast::Sender<OrderBookUpdate>>,
}

impl OrderBookCache {
    /// Create a new cache without notifications.
    #[must_use]
    pub fn new() -> Self {
        Self {
            books: RwLock::new(HashMap::new()),
            tx: None,
        }
    }

    /// Create a new cache with broadcast notifications.
    ///
    /// Returns the cache and a receiver for subscribing to updates.
    /// Additional receivers can be created via `subscribe()`.
    #[must_use]
    pub fn with_notifications(capacity: usize) -> (Self, broadcast::Receiver<OrderBookUpdate>) {
        let (tx, rx) = broadcast::channel(capacity);
        let cache = Self {
            books: RwLock::new(HashMap::new()),
            tx: Some(tx),
        };
        (cache, rx)
    }

    /// Subscribe to order book update notifications.
    ///
    /// Returns `None` if the cache was created without notifications.
    #[must_use]
    pub fn subscribe(&self) -> Option<broadcast::Receiver<OrderBookUpdate>> {
        self.tx.as_ref().map(|tx| tx.subscribe())
    }

    /// Update order book in the cache and notify subscribers.
    pub fn update(&self, book: OrderBook) {
        let token_id = book.token_id().clone();
        self.books.write().insert(token_id.clone(), book);

        // Notify subscribers (ignore send errors - no receivers is fine)
        if let Some(ref tx) = self.tx {
            let _ = tx.send(OrderBookUpdate { token_id });
        }
    }

    /// Get a snapshot of an order book.
    #[must_use]
    pub fn get(&self, token_id: &TokenId) -> Option<OrderBook> {
        self.books.read().get(token_id).cloned()
    }

    /// Get snapshots of two order books atomically.
    #[must_use]
    pub fn get_pair(
        &self,
        token_a: &TokenId,
        token_b: &TokenId,
    ) -> (Option<OrderBook>, Option<OrderBook>) {
        let books = self.books.read();
        (books.get(token_a).cloned(), books.get(token_b).cloned())
    }

    /// Get snapshots of multiple order books atomically.
    #[must_use]
    pub fn get_many(&self, token_ids: &[TokenId]) -> Vec<Option<OrderBook>> {
        let books = self.books.read();
        token_ids.iter().map(|id| books.get(id).cloned()).collect()
    }

    /// Number of books in cache.
    #[must_use]
    pub fn len(&self) -> usize {
        self.books.read().len()
    }

    /// Returns true if the cache is empty.
    #[must_use]
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
    use crate::core::domain::PriceLevel;
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

    #[test]
    fn test_get_many() {
        let cache = OrderBookCache::new();
        let tokens: Vec<TokenId> = (0..3)
            .map(|i| TokenId::from(format!("token-{i}")))
            .collect();

        for (i, token) in tokens.iter().enumerate() {
            let price = rust_decimal::Decimal::from(i as u32 + 1) / rust_decimal::Decimal::from(10);
            let book = OrderBook::with_levels(
                token.clone(),
                vec![],
                vec![PriceLevel::new(price, dec!(100))],
            );
            cache.update(book);
        }

        let results = cache.get_many(&tokens);
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.is_some()));
    }

    #[tokio::test]
    async fn test_with_notifications() {
        let (cache, mut rx) = OrderBookCache::with_notifications(16);
        let token_id = TokenId::from("test-token");

        let book = OrderBook::with_levels(
            token_id.clone(),
            vec![PriceLevel::new(dec!(0.45), dec!(100))],
            vec![],
        );

        cache.update(book);

        let update = rx.recv().await.unwrap();
        assert_eq!(update.token_id.as_str(), "test-token");
    }

    #[test]
    fn test_subscribe() {
        let (cache, _rx) = OrderBookCache::with_notifications(16);
        let rx2 = cache.subscribe();
        assert!(rx2.is_some());

        let cache_no_notify = OrderBookCache::new();
        assert!(cache_no_notify.subscribe().is_none());
    }
}
