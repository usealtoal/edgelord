//! Polymarket exchange integration.

mod approval;
mod client;
mod config;
mod dedup;
mod executor;
mod filter;
mod message;
mod response;
mod scorer;
mod websocket;

pub use approval::{PolymarketApproval, SweepResult};
pub use client::PolymarketClient;
pub use config::{PolymarketExchangeConfig, POLYMARKET_PAYOUT};
pub use dedup::PolymarketDeduplicator;
pub use executor::PolymarketExecutor;
pub use filter::PolymarketFilter;
pub use message::{
    PolymarketBookMessage, PolymarketTaggedMessage, PolymarketWsMessage, PolymarketWsPriceLevel,
};
pub use response::{PolymarketMarket, PolymarketToken};
pub use scorer::PolymarketScorer;
pub use websocket::{PolymarketDataStream, PolymarketWebSocketHandler};
