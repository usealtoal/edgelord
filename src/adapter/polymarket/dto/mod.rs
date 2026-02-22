//! Polymarket data transfer objects.
//!
//! Contains types for API and WebSocket communication:
//! - WebSocket messages (subscriptions, book snapshots, price changes)
//! - REST API responses (market data, tokens)

mod message;
mod response;

pub use message::{
    PolymarketBookMessage, PolymarketSubscribeMessage, PolymarketTaggedMessage, PolymarketWsMessage,
    PolymarketWsPriceLevel,
};
pub use response::{GammaMarket, PolymarketMarket, PolymarketMarketsResponse, PolymarketToken};
