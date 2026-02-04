//! Polymarket exchange integration.

mod client;
mod config;
mod executor;
mod messages;
mod scorer;
mod types;
mod websocket;

pub use client::PolymarketClient;
pub use config::{PolymarketExchangeConfig, POLYMARKET_PAYOUT};
pub use executor::PolymarketExecutor;
pub use messages::{PolymarketBookMessage, PolymarketWsMessage, PolymarketWsPriceLevel};
pub use scorer::PolymarketScorer;
pub use types::{PolymarketMarket, PolymarketToken};
pub use websocket::{PolymarketDataStream, PolymarketWebSocketHandler};
