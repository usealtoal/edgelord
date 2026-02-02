use parking_lot::RwLock;
use std::collections::HashMap;

use crate::types::{OrderBook, PriceLevel, TokenId};
use crate::websocket::BookMessage;

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

    /// Update order book from WebSocket message
    pub fn update(&self, msg: &BookMessage) {
        let token_id = TokenId::from(msg.asset_id.clone());

        let bids: Vec<PriceLevel> = msg
            .bids
            .iter()
            .filter_map(|pl| {
                Some(PriceLevel {
                    price: pl.price.parse().ok()?,
                    size: pl.size.parse().ok()?,
                })
            })
            .collect();

        let asks: Vec<PriceLevel> = msg
            .asks
            .iter()
            .filter_map(|pl| {
                Some(PriceLevel {
                    price: pl.price.parse().ok()?,
                    size: pl.size.parse().ok()?,
                })
            })
            .collect();

        let book = OrderBook {
            token_id: token_id.clone(),
            bids,
            asks,
        };

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

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for OrderBookCache {
    fn default() -> Self {
        Self::new()
    }
}
