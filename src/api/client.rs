use reqwest::Client;
use tracing::{debug, info};

use crate::error::Result;
use super::types::{Market, MarketsResponse};

pub struct ApiClient {
    client: Client,
    base_url: String,
}

impl ApiClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    /// Fetch active markets, limited to a reasonable number for initial testing
    pub async fn get_active_markets(&self, limit: usize) -> Result<Vec<Market>> {
        let url = format!("{}/markets?active=true&closed=false&limit={}", self.base_url, limit);

        info!(url = %url, "Fetching active markets");

        let response: MarketsResponse = self.client
            .get(&url)
            .send()
            .await?
            .json()
            .await?;

        let markets = response.data.unwrap_or_default();
        debug!(count = markets.len(), "Fetched markets");

        Ok(markets)
    }
}
