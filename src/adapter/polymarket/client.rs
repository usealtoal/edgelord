//! Polymarket REST API client.
//!
//! Provides HTTP client functionality for interacting with the Polymarket
//! CLOB API to fetch market data and metadata.

use reqwest::Client;
use tracing::{debug, info};

use super::types::{Market, MarketsResponse};
use crate::error::Result;

/// HTTP client for the Polymarket REST API.
///
/// Handles fetching market data from the Polymarket CLOB (Central Limit Order Book)
/// API endpoints.
pub struct PolymarketClient {
    client: Client,
    base_url: String,
}

impl PolymarketClient {
    /// Create a new Polymarket client with the given base URL.
    ///
    /// # Arguments
    ///
    /// * `base_url` - The base URL for the Polymarket CLOB API
    ///   (e.g., `https://clob.polymarket.com`)
    #[must_use] 
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
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

        let response: MarketsResponse = self.client.get(&url).send().await?.json().await?;

        let markets = response.data.unwrap_or_default();
        debug!(count = markets.len(), "Fetched markets");

        Ok(markets)
    }
}
