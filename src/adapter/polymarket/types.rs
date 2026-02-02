//! Polymarket API response types.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct MarketsResponse {
    pub data: Option<Vec<Market>>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Market {
    pub condition_id: String,
    pub question: Option<String>,
    pub tokens: Vec<Token>,
    pub active: bool,
    pub closed: bool,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Token {
    pub token_id: String,
    pub outcome: String,
    pub price: Option<f64>,
}

impl Market {
    #[allow(dead_code)]
    #[must_use] 
    pub fn token_ids(&self) -> Vec<String> {
        self.tokens.iter().map(|t| t.token_id.clone()).collect()
    }
}
