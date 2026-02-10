//! Polymarket exchange integration.

mod approval;
mod client;
mod config;
mod dedup;
mod executor;
mod filter;
mod message;
// TODO: connection pool for multi-WS support (planned)
mod response;
mod scorer;
mod websocket;

pub use approval::{PolymarketApproval, SweepResult};
pub use client::PolymarketClient;
pub use config::{PolymarketExchangeConfig, POLYMARKET_PAYOUT};
pub use dedup::PolymarketDeduplicator;
pub use executor::PolymarketExecutor;
pub use filter::PolymarketFilter;
pub use message::{PolymarketBookMessage, PolymarketWsMessage, PolymarketWsPriceLevel};
// pub use pool::ConnectionPool;
pub use response::{PolymarketMarket, PolymarketToken};
pub use scorer::PolymarketScorer;
pub use websocket::{PolymarketDataStream, PolymarketWebSocketHandler};
