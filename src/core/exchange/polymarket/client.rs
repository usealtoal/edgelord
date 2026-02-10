//! Polymarket REST API client.
//!
//! Supports two API surfaces:
//! - **CLOB API** (`clob.polymarket.com`) — order execution, order book queries
//! - **Gamma API** (`gamma-api.polymarket.com`) — market discovery with
//!   volume, liquidity, and outcome metadata
//!
//! Market discovery uses the Gamma API for richer data (volume/liquidity).
//! All other operations (WS streaming, order execution) use the CLOB API.

use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client as HttpClient;
use tokio::time::sleep;
use tracing::{debug, info, warn};

use super::response::{GammaMarket, PolymarketMarket, PolymarketMarketsResponse};
use crate::app::PolymarketConfig;
use crate::core::exchange::{MarketFetcher, MarketInfo, OutcomeInfo};
use crate::error::Result;

/// HTTP client for the Polymarket REST APIs.
///
/// Handles fetching market data from both the CLOB and Gamma APIs.
/// Market discovery uses Gamma (richer metadata); trading uses CLOB.
pub struct PolymarketClient {
    http: HttpClient,
    /// CLOB API base URL (order execution, order book).
    base_url: String,
    /// Gamma API base URL (market discovery, volume/liquidity).
    gamma_url: String,
    retry_max_attempts: u32,
    retry_backoff_ms: u64,
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
            http: HttpClient::new(),
            gamma_url: "https://gamma-api.polymarket.com".into(),
            base_url,
            retry_max_attempts: 1,
            retry_backoff_ms: 0,
        }
    }

    #[must_use]
    pub fn from_config(config: &PolymarketConfig) -> Self {
        let http = HttpClient::builder()
            .timeout(Duration::from_millis(config.http.timeout_ms))
            .connect_timeout(Duration::from_millis(config.http.connect_timeout_ms))
            .build()
            .unwrap_or_else(|err| {
                warn!(error = %err, "Failed to build HTTP client, using defaults");
                HttpClient::new()
            });

        Self {
            http,
            base_url: config.api_url.clone(),
            gamma_url: config.gamma_api_url.clone(),
            retry_max_attempts: config.http.retry_max_attempts,
            retry_backoff_ms: config.http.retry_backoff_ms,
        }
    }

    async fn get_with_retry<T>(&self, url: &str) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let mut attempt = 0;
        let max_attempts = self.retry_max_attempts.max(1);

        loop {
            attempt += 1;
            let response = self.http.get(url).send().await;
            let response = match response {
                Ok(response) => response,
                Err(err) => {
                    if attempt >= max_attempts || !Self::should_retry(&err) {
                        return Err(err.into());
                    }
                    self.backoff(attempt, max_attempts, &err).await;
                    continue;
                }
            };

            let response = match response.error_for_status() {
                Ok(response) => response,
                Err(err) => return Err(err.into()),
            };

            let parsed = response.json::<T>().await;
            match parsed {
                Ok(parsed) => return Ok(parsed),
                Err(err) => {
                    if attempt >= max_attempts || !Self::should_retry(&err) {
                        return Err(err.into());
                    }
                    self.backoff(attempt, max_attempts, &err).await;
                }
            }
        }
    }

    fn should_retry(err: &reqwest::Error) -> bool {
        err.is_timeout() || err.is_connect()
    }

    async fn backoff(&self, attempt: u32, max_attempts: u32, err: &reqwest::Error) {
        warn!(
            attempt,
            max_attempts,
            error = %err,
            "HTTP request failed, retrying"
        );
        if self.retry_backoff_ms > 0 {
            sleep(Duration::from_millis(self.retry_backoff_ms)).await;
        }
    }

    /// Fetch active markets from the CLOB API.
    ///
    /// Returns raw market data without volume/liquidity metadata.
    /// Prefer [`get_gamma_markets`] for market discovery.
    pub async fn get_active_markets(&self, limit: usize) -> Result<Vec<PolymarketMarket>> {
        let url = format!(
            "{}/markets?active=true&closed=false&limit={}",
            self.base_url, limit
        );

        info!(url = %url, "Fetching active markets (CLOB)");

        let response: PolymarketMarketsResponse = self.get_with_retry(&url).await?;

        let markets = response.data.unwrap_or_default();
        debug!(count = markets.len(), "Fetched markets from CLOB");

        Ok(markets)
    }

    /// Fetch active markets from the Gamma API.
    ///
    /// Returns market data with volume, liquidity, and outcome metadata.
    /// Used for market discovery and filtering.
    pub async fn get_gamma_markets(&self, limit: usize) -> Result<Vec<GammaMarket>> {
        let url = format!(
            "{}/markets?active=true&closed=false&limit={}",
            self.gamma_url, limit
        );

        info!(url = %url, "Fetching active markets (Gamma)");

        let markets: Vec<GammaMarket> = self.get_with_retry(&url).await?;
        debug!(count = markets.len(), "Fetched markets from Gamma");

        Ok(markets)
    }
}

#[async_trait]
impl MarketFetcher for PolymarketClient {
    async fn get_markets(&self, limit: usize) -> Result<Vec<MarketInfo>> {
        let markets = self.get_gamma_markets(limit).await?;
        Ok(markets.into_iter().map(MarketInfo::from).collect())
    }

    fn exchange_name(&self) -> &'static str {
        "Polymarket"
    }
}

// ---------------------------------------------------------------------------
// MarketInfo conversions
// ---------------------------------------------------------------------------

impl From<PolymarketMarket> for MarketInfo {
    fn from(m: PolymarketMarket) -> Self {
        Self {
            id: m.condition_id,
            question: m.question.unwrap_or_default(),
            outcomes: m
                .tokens
                .into_iter()
                .map(|t| OutcomeInfo {
                    token_id: t.token_id,
                    name: t.outcome,
                    price: t.price,
                })
                .collect(),
            active: m.active && !m.closed,
            volume_24h: m.volume_24h,
            liquidity: m.liquidity,
        }
    }
}

impl From<GammaMarket> for MarketInfo {
    fn from(m: GammaMarket) -> Self {
        let token_ids = m.token_ids();
        let names = m.outcome_names();
        let prices = m.outcome_prices();

        let outcomes = token_ids
            .into_iter()
            .enumerate()
            .map(|(i, token_id)| OutcomeInfo {
                token_id,
                name: names.get(i).cloned().unwrap_or_default(),
                price: prices.get(i).copied(),
            })
            .collect();

        Self {
            id: m.condition_id,
            question: m.question.unwrap_or_default(),
            outcomes,
            active: m.active && !m.closed,
            volume_24h: m.volume_24hr,
            liquidity: m.liquidity_num,
        }
    }
}
