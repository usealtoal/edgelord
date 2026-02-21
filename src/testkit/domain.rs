//! Builders for domain primitives used across tests.
//!
//! Provides concise factory functions for [`TokenId`], [`MarketId`],
//! [`MarketEvent`], and related types so tests focus on assertions
//! rather than construction boilerplate.

use crate::domain::{MarketId, OrderBook, TokenId};
use crate::runtime::exchange::MarketEvent;

/// Generate `n` token IDs named `t0`, `t1`, ..., `t{n-1}`.
pub fn make_tokens(n: usize) -> Vec<TokenId> {
    (0..n).map(|i| TokenId::from(format!("t{i}"))).collect()
}

/// Create a [`TokenId`] from a string.
pub fn token(id: &str) -> TokenId {
    TokenId::from(id.to_string())
}

/// Create a [`MarketId`] from a string.
pub fn market_id(id: &str) -> MarketId {
    MarketId::from(id.to_string())
}

/// Create an [`OrderBookSnapshot`](MarketEvent::OrderBookSnapshot) event
/// with an empty order book.
pub fn snapshot_event(token: &str) -> MarketEvent {
    MarketEvent::OrderBookSnapshot {
        token_id: TokenId::from(token.to_string()),
        book: OrderBook::new(TokenId::from(token.to_string())),
    }
}

/// Create a [`Disconnected`](MarketEvent::Disconnected) event.
pub fn disconnect_event(reason: &str) -> MarketEvent {
    MarketEvent::Disconnected {
        reason: reason.to_string(),
    }
}
