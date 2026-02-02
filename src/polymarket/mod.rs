//! Polymarket exchange integration.

mod client;
mod messages;
mod registry;
mod types;
mod websocket;

pub use client::PolymarketClient;
pub use messages::{BookMessage, WsMessage, WsPriceLevel};
pub use registry::MarketRegistry;
pub use types::{Market, Token};
pub use websocket::WebSocketHandler;
