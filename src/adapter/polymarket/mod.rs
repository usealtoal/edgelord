//! Polymarket exchange integration.

mod client;
mod executor;
mod messages;
mod registry;
mod types;
mod websocket;

pub use client::Client;
pub use executor::{ArbitrageExecutionResult, Executor};
pub use messages::{BookMessage, WsMessage, WsPriceLevel};
pub use registry::MarketRegistry;
// Re-export for future use
#[allow(unused_imports)]
pub use types::{Market, Token};
pub use websocket::{DataStream, WebSocketHandler};
