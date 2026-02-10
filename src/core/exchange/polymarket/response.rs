//! Polymarket API response types.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PolymarketMarketsResponse {
    pub data: Option<Vec<PolymarketMarket>>,
    /// Cursor for pagination (reserved for future use).
    #[allow(dead_code)]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PolymarketMarket {
    pub condition_id: String,
    pub question: Option<String>,
    pub tokens: Vec<PolymarketToken>,
    pub active: bool,
    pub closed: bool,
    /// 24-hour trading volume in USD (from Gamma API or CLOB extended fields).
    #[serde(default, alias = "volume_num_24hr")]
    pub volume_24h: Option<f64>,
    /// Current liquidity depth in USD.
    #[serde(default)]
    pub liquidity: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct PolymarketToken {
    pub token_id: String,
    pub outcome: String,
    pub price: Option<f64>,
}

impl PolymarketMarket {
    #[must_use]
    pub fn token_ids(&self) -> Vec<String> {
        self.tokens.iter().map(|t| t.token_id.clone()).collect()
    }
}
