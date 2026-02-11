//! Polymarket API response types.
//!
//! Two API surfaces:
//! - **CLOB API** (`clob.polymarket.com`) — order execution, order book, WS streaming.
//!   Uses [`PolymarketMarket`] and [`PolymarketMarketsResponse`].
//! - **Gamma API** (`gamma-api.polymarket.com`) — market discovery, metadata,
//!   volume/liquidity stats. Uses [`GammaMarket`].

use serde::Deserialize;
use tracing::debug;

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

// ---------------------------------------------------------------------------
// Gamma API types
// ---------------------------------------------------------------------------

/// Market data from the Gamma API.
///
/// The Gamma API provides richer metadata than the CLOB API, including
/// trading volume, liquidity, and outcome prices. Used for market discovery
/// and filtering.
///
/// Response format: flat JSON array (no wrapper object).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GammaMarket {
    /// Condition ID (same as CLOB's `condition_id`).
    pub condition_id: String,
    /// Market question.
    #[serde(default)]
    pub question: Option<String>,
    /// Whether the market is active.
    #[serde(default)]
    pub active: bool,
    /// Whether the market is closed.
    #[serde(default)]
    pub closed: bool,
    /// JSON-encoded outcome names (e.g., `["Yes", "No"]`).
    #[serde(default)]
    pub outcomes: Option<String>,
    /// JSON-encoded outcome prices (e.g., `["0.65", "0.35"]`).
    #[serde(default)]
    pub outcome_prices: Option<String>,
    /// JSON-encoded CLOB token IDs.
    #[serde(default)]
    pub clob_token_ids: Option<String>,
    /// 24-hour trading volume in USD.
    #[serde(default, alias = "volume24hr")]
    pub volume_24hr: Option<f64>,
    /// Total all-time volume in USD.
    #[serde(default, alias = "volumeNum")]
    pub volume_num: Option<f64>,
    /// Current liquidity depth in USD.
    #[serde(default, alias = "liquidityNum")]
    pub liquidity_num: Option<f64>,
}

impl GammaMarket {
    /// Parse the JSON-encoded CLOB token IDs.
    pub fn token_ids(&self) -> Vec<String> {
        self.clob_token_ids
            .as_deref()
            .and_then(|s| {
                serde_json::from_str::<Vec<String>>(s)
                    .map_err(|e| {
                        debug!(
                            error = %e,
                            raw = %s,
                            condition_id = %self.condition_id,
                            "Failed to parse clob_token_ids"
                        );
                    })
                    .ok()
            })
            .unwrap_or_default()
    }

    /// Parse the JSON-encoded outcome names.
    pub fn outcome_names(&self) -> Vec<String> {
        self.outcomes
            .as_deref()
            .and_then(|s| {
                serde_json::from_str::<Vec<String>>(s)
                    .map_err(|e| {
                        debug!(
                            error = %e,
                            raw = %s,
                            condition_id = %self.condition_id,
                            "Failed to parse outcomes"
                        );
                    })
                    .ok()
            })
            .unwrap_or_default()
    }

    /// Parse the JSON-encoded outcome prices.
    pub fn outcome_prices(&self) -> Vec<f64> {
        self.outcome_prices
            .as_deref()
            .and_then(|s| {
                serde_json::from_str::<Vec<String>>(s)
                    .map_err(|e| {
                        debug!(
                            error = %e,
                            raw = %s,
                            condition_id = %self.condition_id,
                            "Failed to parse outcome_prices"
                        );
                    })
                    .ok()
            })
            .map(|v| v.iter().filter_map(|p| p.parse::<f64>().ok()).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gamma_market_deserializes_from_api_response() {
        let json = r#"{
            "conditionId": "0xabc123",
            "question": "Will X happen?",
            "active": true,
            "closed": false,
            "outcomes": "[\"Yes\", \"No\"]",
            "outcomePrices": "[\"0.65\", \"0.35\"]",
            "clobTokenIds": "[\"token-yes\", \"token-no\"]",
            "volume24hr": 8456.03,
            "volumeNum": 1081783.60,
            "liquidityNum": 14854.96
        }"#;

        let market: GammaMarket = serde_json::from_str(json).unwrap();

        assert_eq!(market.condition_id, "0xabc123");
        assert_eq!(market.question.as_deref(), Some("Will X happen?"));
        assert!(market.active);
        assert!(!market.closed);
        assert!((market.volume_24hr.unwrap() - 8456.03).abs() < 0.01);
        assert!((market.liquidity_num.unwrap() - 14854.96).abs() < 0.01);

        let ids = market.token_ids();
        assert_eq!(ids, vec!["token-yes", "token-no"]);

        let names = market.outcome_names();
        assert_eq!(names, vec!["Yes", "No"]);

        let prices = market.outcome_prices();
        assert_eq!(prices.len(), 2);
        assert!((prices[0] - 0.65).abs() < 0.01);
        assert!((prices[1] - 0.35).abs() < 0.01);
    }

    #[test]
    fn gamma_market_handles_missing_optional_fields() {
        let json = r#"{
            "conditionId": "0xdef456",
            "active": true,
            "closed": false
        }"#;

        let market: GammaMarket = serde_json::from_str(json).unwrap();

        assert_eq!(market.condition_id, "0xdef456");
        assert!(market.question.is_none());
        assert!(market.volume_24hr.is_none());
        assert!(market.liquidity_num.is_none());
        assert!(market.token_ids().is_empty());
        assert!(market.outcome_names().is_empty());
        assert!(market.outcome_prices().is_empty());
    }

    #[test]
    fn gamma_market_converts_to_market_info() {
        use crate::core::exchange::MarketInfo;

        let json = r#"{
            "conditionId": "0xabc",
            "question": "Test?",
            "active": true,
            "closed": false,
            "outcomes": "[\"Yes\", \"No\"]",
            "outcomePrices": "[\"0.70\", \"0.30\"]",
            "clobTokenIds": "[\"t1\", \"t2\"]",
            "volume24hr": 5000.0,
            "liquidityNum": 2000.0
        }"#;

        let gamma: GammaMarket = serde_json::from_str(json).unwrap();
        let info = MarketInfo::from(gamma);

        assert_eq!(info.id, "0xabc");
        assert_eq!(info.question, "Test?");
        assert!(info.active);
        assert_eq!(info.outcomes.len(), 2);
        assert_eq!(info.outcomes[0].token_id, "t1");
        assert_eq!(info.outcomes[0].name, "Yes");
        assert!((info.outcomes[0].price.unwrap() - 0.70).abs() < 0.01);
        assert!((info.volume_24h.unwrap() - 5000.0).abs() < 0.01);
        assert!((info.liquidity.unwrap() - 2000.0).abs() < 0.01);
    }
}
