//! Polymarket exchange integration.

mod client;
mod config;
mod executor;
mod messages;
mod registry;
mod types;
mod websocket;

pub use client::Client;
pub use config::{PolymarketExchangeConfig, POLYMARKET_PAYOUT};
pub use executor::Executor;
pub use messages::{BookMessage, WsMessage, WsPriceLevel};
pub use registry::MarketRegistry;
pub use types::{Market, Token};
pub use websocket::{DataStream, WebSocketHandler};
