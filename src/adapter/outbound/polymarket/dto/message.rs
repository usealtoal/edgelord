//! Polymarket WebSocket message types.
//!
//! Polymarket WebSocket messages are sent as JSON arrays containing one or more
//! book snapshot objects. Each object contains order book data for a single token.
//!
//! Example message format:
//! ```json
//! [{"market":"0x...","asset_id":"123...","timestamp":"1234","hash":"abc","bids":[...],"asks":[...]}]
//! ```

use serde::{Deserialize, Serialize};

use crate::domain::{book::Book, book::PriceLevel, id::TokenId};

/// Subscription request sent to Polymarket WebSocket
#[derive(Debug, Serialize)]
pub struct PolymarketSubscribeMessage {
    pub assets_ids: Vec<String>,
    #[serde(rename = "type")]
    pub msg_type: String,
}

impl PolymarketSubscribeMessage {
    pub fn new(asset_ids: Vec<String>) -> Self {
        Self {
            assets_ids: asset_ids,
            msg_type: "market".into(),
        }
    }
}

/// Messages received from Polymarket WebSocket.
///
/// Messages arrive as a JSON array of book snapshots. Each snapshot contains
/// the full order book state for a single token.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum PolymarketWsMessage {
    /// Array of book snapshots (primary message format).
    Books(Vec<PolymarketBookMessage>),

    /// Unknown or unparseable message.
    Unknown(serde_json::Value),
}

#[derive(Debug, Deserialize)]
pub struct PolymarketBookMessage {
    pub asset_id: String,
    pub market: Option<String>,
    pub bids: Vec<PolymarketWsPriceLevel>,
    pub asks: Vec<PolymarketWsPriceLevel>,
    pub timestamp: Option<String>,
    pub hash: Option<String>,
}

impl PolymarketBookMessage {
    /// Convert this WebSocket message to a domain `Book`
    #[must_use]
    pub fn to_orderbook(&self) -> Book {
        let token_id = TokenId::from(self.asset_id.clone());
        let bids = Self::parse_levels(&self.bids);
        let asks = Self::parse_levels(&self.asks);
        Book::with_levels(token_id, bids, asks)
    }

    fn parse_levels(levels: &[PolymarketWsPriceLevel]) -> Vec<PriceLevel> {
        levels
            .iter()
            .filter_map(|pl| {
                Some(PriceLevel::new(
                    pl.price.parse().ok()?,
                    pl.size.parse().ok()?,
                ))
            })
            .collect()
    }
}

/// Price level as received from WebSocket (strings, not decimals)
#[derive(Debug, Clone, Deserialize)]
pub struct PolymarketWsPriceLevel {
    pub price: String,
    pub size: String,
}
