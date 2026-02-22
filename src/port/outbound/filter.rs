//! Market filtering and scoring ports.
//!
//! Traits for filtering markets for subscription eligibility and
//! scoring them for prioritization.

use async_trait::async_trait;

use crate::domain::{score::MarketScore, score::ScoreWeights};
use crate::error::Result;
use crate::port::outbound::exchange::MarketInfo;

/// Configuration for market filtering.
#[derive(Debug, Clone, PartialEq)]
pub struct MarketFilterConfig {
    /// Maximum number of markets to consider for subscription.
    pub max_markets: usize,
    /// Maximum number of active subscriptions allowed.
    pub max_subscriptions: usize,
    /// Minimum 24-hour trading volume (in USD).
    pub min_volume_24h: f64,
    /// Minimum liquidity depth (in USD).
    pub min_liquidity: f64,
    /// Maximum bid-ask spread as a percentage (e.g., 5.0 = 5%).
    pub max_spread_pct: f64,
    /// Whether to include binary (YES/NO) markets.
    pub include_binary: bool,
    /// Whether to include multi-outcome markets.
    pub include_multi_outcome: bool,
    /// Maximum number of outcomes allowed in a market.
    pub max_outcomes: usize,
}

impl Default for MarketFilterConfig {
    fn default() -> Self {
        Self {
            max_markets: 1000,
            max_subscriptions: 100,
            min_volume_24h: 1000.0,
            min_liquidity: 500.0,
            max_spread_pct: 10.0,
            include_binary: true,
            include_multi_outcome: true,
            max_outcomes: 10,
        }
    }
}

/// Filters markets for subscription eligibility.
pub trait MarketFilter: Send + Sync {
    /// Check if a single market is eligible for subscription.
    fn is_eligible(&self, market: &MarketInfo) -> bool;

    /// Filter a slice of markets, returning only eligible ones.
    fn filter(&self, markets: &[MarketInfo]) -> Vec<MarketInfo> {
        markets
            .iter()
            .filter(|m| self.is_eligible(m))
            .cloned()
            .collect()
    }

    /// Get the filter configuration.
    fn config(&self) -> &MarketFilterConfig;

    /// Get the exchange name for logging and debugging.
    fn exchange_name(&self) -> &'static str;
}

/// Scores markets for subscription prioritization.
#[async_trait]
pub trait MarketScorer: Send + Sync {
    /// Score a single market for subscription prioritization.
    async fn score(&self, market: &MarketInfo) -> Result<MarketScore>;

    /// Score multiple markets in batch.
    async fn score_batch(&self, markets: &[MarketInfo]) -> Result<Vec<MarketScore>> {
        let mut scores = Vec::with_capacity(markets.len());
        for market in markets {
            scores.push(self.score(market).await?);
        }
        Ok(scores)
    }

    /// Get the scoring weights used by this scorer.
    fn weights(&self) -> &ScoreWeights;

    /// Get the exchange name for logging and debugging.
    fn exchange_name(&self) -> &'static str;
}
