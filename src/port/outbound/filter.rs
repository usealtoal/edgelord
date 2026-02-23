//! Market filtering and scoring ports.
//!
//! Defines traits for filtering markets by eligibility criteria and scoring
//! them for subscription prioritization.
//!
//! # Overview
//!
//! - [`MarketFilter`]: Filter markets by volume, liquidity, spread, etc.
//! - [`MarketScorer`]: Score markets for prioritized subscription
//! - [`MarketFilterConfig`]: Configuration for filtering criteria

use async_trait::async_trait;

use crate::domain::{score::MarketScore, score::ScoreWeights};
use crate::error::Result;
use crate::port::outbound::exchange::MarketInfo;

/// Configuration for market filtering criteria.
#[derive(Debug, Clone, PartialEq)]
pub struct MarketFilterConfig {
    /// Maximum number of markets to consider for subscription.
    pub max_markets: usize,

    /// Maximum number of active subscriptions allowed.
    pub max_subscriptions: usize,

    /// Minimum 24-hour trading volume in USD.
    pub min_volume_24h: f64,

    /// Minimum liquidity depth in USD.
    pub min_liquidity: f64,

    /// Maximum bid-ask spread as a percentage (e.g., 5.0 = 5%).
    pub max_spread_pct: f64,

    /// Whether to include binary (two-outcome) markets.
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

/// Filter for determining market subscription eligibility.
///
/// # Thread Safety
///
/// Implementations must be thread-safe (`Send + Sync`).
pub trait MarketFilter: Send + Sync {
    /// Return `true` if the market is eligible for subscription.
    ///
    /// # Arguments
    ///
    /// * `market` - Market information to evaluate.
    fn is_eligible(&self, market: &MarketInfo) -> bool;

    /// Filter a collection of markets, returning only eligible ones.
    ///
    /// # Arguments
    ///
    /// * `markets` - Markets to filter.
    ///
    /// Default implementation calls `is_eligible` for each market.
    fn filter(&self, markets: &[MarketInfo]) -> Vec<MarketInfo> {
        markets
            .iter()
            .filter(|m| self.is_eligible(m))
            .cloned()
            .collect()
    }

    /// Return the filter configuration.
    fn config(&self) -> &MarketFilterConfig;

    /// Return the exchange name for logging and debugging.
    fn exchange_name(&self) -> &'static str;
}

/// Scorer for prioritizing market subscriptions.
///
/// Markets with higher scores are prioritized when subscription slots are
/// limited.
///
/// # Thread Safety
///
/// Implementations must be thread-safe (`Send + Sync`).
#[async_trait]
pub trait MarketScorer: Send + Sync {
    /// Compute a priority score for a market.
    ///
    /// # Arguments
    ///
    /// * `market` - Market to score.
    ///
    /// # Errors
    ///
    /// Returns an error if scoring fails (e.g., missing data).
    async fn score(&self, market: &MarketInfo) -> Result<MarketScore>;

    /// Score multiple markets in batch.
    ///
    /// # Arguments
    ///
    /// * `markets` - Markets to score.
    ///
    /// Default implementation scores markets sequentially.
    ///
    /// # Errors
    ///
    /// Returns an error if any market fails to score.
    async fn score_batch(&self, markets: &[MarketInfo]) -> Result<Vec<MarketScore>> {
        let mut scores = Vec::with_capacity(markets.len());
        for market in markets {
            scores.push(self.score(market).await?);
        }
        Ok(scores)
    }

    /// Return the scoring weights used by this scorer.
    fn weights(&self) -> &ScoreWeights;

    /// Return the exchange name for logging and debugging.
    fn exchange_name(&self) -> &'static str;
}
