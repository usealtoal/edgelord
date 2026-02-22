//! Market scoring trait for subscription prioritization.
//!
//! Exchanges implement [`MarketScorer`] to score markets based on their characteristics,
//! enabling adaptive subscription management to prioritize high-value markets.

use async_trait::async_trait;

use crate::runtime::{MarketScore, ScoreWeights};
use crate::error::Result;

use super::MarketInfo;

/// Scores markets for subscription prioritization.
///
/// Implementations analyze market characteristics (liquidity, spread, activity, etc.)
/// and compute a composite score used to determine subscription priority. Higher-scored
/// markets are prioritized when subscription slots are limited.
///
/// # Example
///
/// ```ignore
/// struct MyExchangeScorer {
///     weights: ScoreWeights,
/// }
///
/// #[async_trait]
/// impl MarketScorer for MyExchangeScorer {
///     async fn score(&self, market: &MarketInfo) -> Result<MarketScore> {
///         // Compute factors based on market characteristics
///         let factors = ScoreFactors::new(
///             compute_liquidity(market),
///             compute_spread(market),
///             compute_opportunity(market),
///             compute_outcome_count(market),
///             compute_activity(market),
///         );
///         Ok(MarketScore::from_factors(
///             MarketId::from(market.id.as_str()),
///             factors,
///             &self.weights,
///         ))
///     }
///
///     fn weights(&self) -> &ScoreWeights {
///         &self.weights
///     }
///
///     fn exchange_name(&self) -> &'static str {
///         "myexchange"
///     }
/// }
/// ```
#[async_trait]
pub trait MarketScorer: Send + Sync {
    /// Score a single market for subscription prioritization.
    ///
    /// Analyzes the market's characteristics and returns a [`MarketScore`] containing
    /// the individual scoring factors and computed composite score.
    ///
    /// # Arguments
    ///
    /// * `market` - Market information to score
    ///
    /// # Errors
    ///
    /// Returns an error if scoring fails (e.g., unable to fetch required data).
    async fn score(&self, market: &MarketInfo) -> Result<MarketScore>;

    /// Score multiple markets in batch.
    ///
    /// Default implementation calls [`score`](Self::score) for each market individually.
    /// Implementations may override this for more efficient batch processing.
    ///
    /// # Arguments
    ///
    /// * `markets` - Slice of markets to score
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

    /// Get the scoring weights used by this scorer.
    ///
    /// Weights determine the relative importance of each scoring factor
    /// when computing the composite score.
    fn weights(&self) -> &ScoreWeights;

    /// Get the exchange name for logging and debugging.
    fn exchange_name(&self) -> &'static str;
}
