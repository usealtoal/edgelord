//! Polymarket exchange integration.

mod approval;
mod client;
mod config;
mod dedup;
mod executor;
mod filter;
mod message;
mod scorer;
mod response;
mod websocket;

pub use approval::PolymarketApproval;
pub use client::PolymarketClient;
pub use config::{PolymarketExchangeConfig, POLYMARKET_PAYOUT};
pub use dedup::PolymarketDeduplicator;
pub use executor::PolymarketExecutor;
pub use filter::PolymarketFilter;
pub use message::{PolymarketBookMessage, PolymarketWsMessage, PolymarketWsPriceLevel};
pub use scorer::PolymarketScorer;
pub use response::{PolymarketMarket, PolymarketToken};
pub use websocket::{PolymarketDataStream, PolymarketWebSocketHandler};
