use serde::Deserialize;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct MarketsResponse {
    pub data: Option<Vec<Market>>,
    pub next_cursor: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Market {
    pub condition_id: String,
    pub question: Option<String>,
    pub tokens: Vec<Token>,
    pub active: bool,
    pub closed: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Token {
    pub token_id: String,
    pub outcome: String,
    pub price: Option<f64>,
}

#[allow(dead_code)]
impl Market {
    pub fn token_ids(&self) -> Vec<String> {
        self.tokens.iter().map(|t| t.token_id.clone()).collect()
    }
}
