//! Polymarket exchange integration.

mod approval;
mod client;
mod config;
mod dedup;
mod executor;
mod filter;
mod scorer;
mod stream;
#[path = "type/mod.rs"]
mod r#type;

pub use approval::{PolymarketApproval, SweepResult};
pub use client::PolymarketClient;
pub use config::{PolymarketExchangeConfig, POLYMARKET_PAYOUT};
pub use dedup::PolymarketDeduplicator;
pub use executor::PolymarketExecutor;
pub use filter::PolymarketFilter;
pub use r#type::{
    PolymarketBookMessage, PolymarketMarket, PolymarketTaggedMessage, PolymarketToken,
    PolymarketWsMessage, PolymarketWsPriceLevel,
};
pub use scorer::PolymarketScorer;
pub use stream::{PolymarketDataStream, PolymarketWebSocketHandler};
