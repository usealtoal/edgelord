//! Polymarket REST API client.
//!
//! Provides HTTP client functionality for interacting with the Polymarket
//! CLOB API to fetch market data and metadata.

use async_trait::async_trait;
use reqwest::Client as HttpClient;
use tracing::{debug, info};

use super::types::{Market, MarketsResponse};
use crate::error::Result;
use super::{MarketFetcher, MarketInfo, OutcomeInfo};

/// HTTP client for the Polymarket REST API.
///
/// Handles fetching market data from the Polymarket CLOB (Central Limit Order Book)
/// API endpoints.
pub struct Client {
    http: HttpClient,
    base_url: String,
}

impl Client {
    /// Create a new Polymarket client with the given base URL.
    ///
    /// # Arguments
    ///
    /// * `base_url` - The base URL for the Polymarket CLOB API
    ///   (e.g., `https://clob.polymarket.com`)
    #[must_use] 
    pub fn new(base_url: String) -> Self {
        Self {
            http: HttpClient::new(),
            base_url,
        }
    }

    /// Fetch active markets from the Polymarket API.
    ///
    /// Returns markets that are currently active and not closed, limited to
    /// the specified count for resource management.
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of markets to fetch
    pub async fn get_active_markets(&self, limit: usize) -> Result<Vec<Market>> {
        let url = format!(
            "{}/markets?active=true&closed=false&limit={}",
            self.base_url, limit
        );

        info!(url = %url, "Fetching active markets");

        let response: MarketsResponse = self.http.get(&url).send().await?.json().await?;

        let markets = response.data.unwrap_or_default();
        debug!(count = markets.len(), "Fetched markets");

        Ok(markets)
    }
}

#[async_trait]
impl MarketFetcher for Client {
    async fn get_markets(&self, limit: usize) -> Result<Vec<MarketInfo>> {
        let markets = self.get_active_markets(limit).await?;
        Ok(markets.into_iter().map(MarketInfo::from).collect())
    }

    fn exchange_name(&self) -> &'static str {
        "Polymarket"
    }
}

impl From<Market> for MarketInfo {
    fn from(m: Market) -> Self {
        Self {
            id: m.condition_id,
            question: m.question.unwrap_or_default(),
            outcomes: m
                .tokens
                .into_iter()
                .map(|t| OutcomeInfo {
                    token_id: t.token_id,
                    name: t.outcome,
                })
                .collect(),
            active: m.active && !m.closed,
        }
    }
}
