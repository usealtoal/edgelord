//! Polymarket WebSocket message types.

use serde::{Deserialize, Serialize};

/// Subscription request sent to Polymarket WebSocket
#[derive(Debug, Serialize)]
pub struct SubscribeMessage {
    pub assets_ids: Vec<String>,
    #[serde(rename = "type")]
    pub msg_type: String,
}

impl SubscribeMessage {
    pub fn new(asset_ids: Vec<String>) -> Self {
        Self {
            assets_ids: asset_ids,
            msg_type: "market".into(),
        }
    }
}

/// Messages received from Polymarket WebSocket
#[derive(Debug, Deserialize)]
#[serde(tag = "event_type")]
pub enum WsMessage {
    #[serde(rename = "book")]
    Book(BookMessage),

    #[serde(rename = "price_change")]
    PriceChange(PriceChangeMessage),

    #[serde(rename = "tick_size_change")]
    TickSizeChange(serde_json::Value),

    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct BookMessage {
    pub asset_id: String,
    pub market: Option<String>,
    pub bids: Vec<WsPriceLevel>,
    pub asks: Vec<WsPriceLevel>,
    pub timestamp: Option<String>,
    pub hash: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PriceChangeMessage {
    pub asset_id: String,
    pub market: Option<String>,
    pub price: Option<String>,
    pub changes: Option<Vec<WsPriceLevel>>,
}

/// Price level as received from WebSocket (strings, not decimals)
#[derive(Debug, Clone, Deserialize)]
pub struct WsPriceLevel {
    pub price: String,
    pub size: String,
}
