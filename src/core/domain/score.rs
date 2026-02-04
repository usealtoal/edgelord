//! Market scoring types for subscription prioritization.
//!
//! These types are used to score and prioritize markets for subscription management.
//! Markets with higher scores should be prioritized for active subscriptions.
//!
//! - [`ScoreFactors`] - Individual scoring factors for a market (0.0-1.0 range)
//! - [`ScoreWeights`] - Weights for combining factors into a composite score
//! - [`MarketScore`] - A market's computed score with its factors

use std::cmp::Ordering;

use super::id::MarketId;

/// Individual scoring factors for a market.
///
/// Each factor is normalized to the 0.0-1.0 range where higher values indicate
/// more desirable characteristics for subscription prioritization.
///
/// # Fields
///
/// - `liquidity` - Market depth and available volume (higher = more liquid)
/// - `spread` - Tightness of bid-ask spread (higher = tighter spread, better)
/// - `opportunity` - Historical arbitrage opportunity frequency (higher = more opportunities)
/// - `outcome_count` - Normalized outcome count factor (higher = more outcomes)
/// - `activity` - Recent trading activity level (higher = more active)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScoreFactors {
    /// Market depth and available volume (0.0-1.0).
    pub liquidity: f64,
    /// Tightness of bid-ask spread (0.0-1.0, higher = tighter).
    pub spread: f64,
    /// Historical arbitrage opportunity frequency (0.0-1.0).
    pub opportunity: f64,
    /// Normalized outcome count factor (0.0-1.0).
    pub outcome_count: f64,
    /// Recent trading activity level (0.0-1.0).
    pub activity: f64,
}

impl ScoreFactors {
    /// Create new score factors.
    ///
    /// All values should be in the 0.0-1.0 range.
    #[must_use]
    pub const fn new(
        liquidity: f64,
        spread: f64,
        opportunity: f64,
        outcome_count: f64,
        activity: f64,
    ) -> Self {
        Self {
            liquidity,
            spread,
            opportunity,
            outcome_count,
            activity,
        }
    }

    /// Compute a weighted composite score from these factors.
    ///
    /// The composite score is the weighted sum of all factors, normalized by
    /// the sum of weights. Returns 0.0 if all weights are zero.
    #[must_use]
    pub fn composite(&self, weights: &ScoreWeights) -> f64 {
        let weighted_sum = self.liquidity * weights.liquidity
            + self.spread * weights.spread
            + self.opportunity * weights.opportunity
            + self.outcome_count * weights.outcome_count
            + self.activity * weights.activity;

        let weight_sum = weights.liquidity
            + weights.spread
            + weights.opportunity
            + weights.outcome_count
            + weights.activity;

        if weight_sum == 0.0 {
            0.0
        } else {
            weighted_sum / weight_sum
        }
    }
}

impl Default for ScoreFactors {
    fn default() -> Self {
        Self {
            liquidity: 0.0,
            spread: 0.0,
            opportunity: 0.0,
            outcome_count: 0.0,
            activity: 0.0,
        }
    }
}

/// Weights for combining score factors into a composite score.
///
/// Higher weights give more importance to that factor when computing
/// the composite score. Weights do not need to sum to 1.0 - they are
/// normalized during composite calculation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScoreWeights {
    /// Weight for liquidity factor.
    pub liquidity: f64,
    /// Weight for spread factor.
    pub spread: f64,
    /// Weight for opportunity factor.
    pub opportunity: f64,
    /// Weight for outcome count factor.
    pub outcome_count: f64,
    /// Weight for activity factor.
    pub activity: f64,
}

impl ScoreWeights {
    /// Create new score weights.
    #[must_use]
    pub const fn new(
        liquidity: f64,
        spread: f64,
        opportunity: f64,
        outcome_count: f64,
        activity: f64,
    ) -> Self {
        Self {
            liquidity,
            spread,
            opportunity,
            outcome_count,
            activity,
        }
    }
}

impl Default for ScoreWeights {
    fn default() -> Self {
        Self {
            liquidity: 1.0,
            spread: 1.0,
            opportunity: 1.0,
            outcome_count: 1.0,
            activity: 1.0,
        }
    }
}

/// A market's computed score with its contributing factors.
///
/// Used for prioritizing which markets to subscribe to. Higher composite
/// scores indicate higher priority markets.
#[derive(Debug, Clone)]
pub struct MarketScore {
    market_id: MarketId,
    factors: ScoreFactors,
    composite: f64,
}

impl MarketScore {
    /// Create a new market score with pre-computed composite.
    #[must_use]
    pub fn new(market_id: MarketId, factors: ScoreFactors, composite: f64) -> Self {
        Self {
            market_id,
            factors,
            composite,
        }
    }

    /// Create a market score by computing the composite from factors and weights.
    #[must_use]
    pub fn from_factors(
        market_id: MarketId,
        factors: ScoreFactors,
        weights: &ScoreWeights,
    ) -> Self {
        let composite = factors.composite(weights);
        Self {
            market_id,
            factors,
            composite,
        }
    }

    /// Get the market ID.
    #[must_use]
    pub fn market_id(&self) -> &MarketId {
        &self.market_id
    }

    /// Get the score factors.
    #[must_use]
    pub const fn factors(&self) -> &ScoreFactors {
        &self.factors
    }

    /// Get the composite score.
    #[must_use]
    pub const fn composite(&self) -> f64 {
        self.composite
    }
}

impl PartialEq for MarketScore {
    fn eq(&self, other: &Self) -> bool {
        self.composite == other.composite
    }
}

impl Eq for MarketScore {}

impl PartialOrd for MarketScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MarketScore {
    /// Compare market scores by composite value.
    ///
    /// Higher composite scores are considered greater (higher priority).
    /// NaN values are treated as less than any other value.
    fn cmp(&self, other: &Self) -> Ordering {
        self.composite
            .partial_cmp(&other.composite)
            .unwrap_or(Ordering::Equal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_market_id(name: &str) -> MarketId {
        MarketId::from(name)
    }

    // --- ScoreFactors tests ---

    #[test]
    fn score_factors_new_stores_values() {
        let factors = ScoreFactors::new(0.1, 0.2, 0.3, 0.4, 0.5);

        assert!((factors.liquidity - 0.1).abs() < f64::EPSILON);
        assert!((factors.spread - 0.2).abs() < f64::EPSILON);
        assert!((factors.opportunity - 0.3).abs() < f64::EPSILON);
        assert!((factors.outcome_count - 0.4).abs() < f64::EPSILON);
        assert!((factors.activity - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn score_factors_default_is_all_zeros() {
        let factors = ScoreFactors::default();

        assert!((factors.liquidity).abs() < f64::EPSILON);
        assert!((factors.spread).abs() < f64::EPSILON);
        assert!((factors.opportunity).abs() < f64::EPSILON);
        assert!((factors.outcome_count).abs() < f64::EPSILON);
        assert!((factors.activity).abs() < f64::EPSILON);
    }

    #[test]
    fn score_factors_composite_with_equal_weights() {
        let factors = ScoreFactors::new(0.2, 0.4, 0.6, 0.8, 1.0);
        let weights = ScoreWeights::new(1.0, 1.0, 1.0, 1.0, 1.0);

        let composite = factors.composite(&weights);

        // (0.2 + 0.4 + 0.6 + 0.8 + 1.0) / 5 = 0.6
        assert!((composite - 0.6).abs() < f64::EPSILON);
    }

    #[test]
    fn score_factors_composite_with_varying_weights() {
        let factors = ScoreFactors::new(1.0, 0.0, 0.0, 0.0, 0.0);
        let weights = ScoreWeights::new(2.0, 1.0, 1.0, 1.0, 1.0);

        let composite = factors.composite(&weights);

        // (1.0 * 2.0) / (2.0 + 1.0 + 1.0 + 1.0 + 1.0) = 2.0 / 6.0 = 0.333...
        assert!((composite - (2.0 / 6.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn score_factors_composite_with_zero_weights_returns_zero() {
        let factors = ScoreFactors::new(1.0, 1.0, 1.0, 1.0, 1.0);
        let weights = ScoreWeights::new(0.0, 0.0, 0.0, 0.0, 0.0);

        let composite = factors.composite(&weights);

        assert!((composite).abs() < f64::EPSILON);
    }

    #[test]
    fn score_factors_composite_single_factor_weighted() {
        let factors = ScoreFactors::new(0.5, 0.0, 0.0, 0.0, 0.0);
        let weights = ScoreWeights::new(1.0, 0.0, 0.0, 0.0, 0.0);

        let composite = factors.composite(&weights);

        // Only liquidity matters: 0.5 * 1.0 / 1.0 = 0.5
        assert!((composite - 0.5).abs() < f64::EPSILON);
    }

    // --- ScoreWeights tests ---

    #[test]
    fn score_weights_new_stores_values() {
        let weights = ScoreWeights::new(0.1, 0.2, 0.3, 0.4, 0.5);

        assert!((weights.liquidity - 0.1).abs() < f64::EPSILON);
        assert!((weights.spread - 0.2).abs() < f64::EPSILON);
        assert!((weights.opportunity - 0.3).abs() < f64::EPSILON);
        assert!((weights.outcome_count - 0.4).abs() < f64::EPSILON);
        assert!((weights.activity - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn score_weights_default_is_all_ones() {
        let weights = ScoreWeights::default();

        assert!((weights.liquidity - 1.0).abs() < f64::EPSILON);
        assert!((weights.spread - 1.0).abs() < f64::EPSILON);
        assert!((weights.opportunity - 1.0).abs() < f64::EPSILON);
        assert!((weights.outcome_count - 1.0).abs() < f64::EPSILON);
        assert!((weights.activity - 1.0).abs() < f64::EPSILON);
    }

    // --- MarketScore tests ---

    #[test]
    fn market_score_new_stores_values() {
        let factors = ScoreFactors::new(0.5, 0.5, 0.5, 0.5, 0.5);
        let score = MarketScore::new(make_market_id("test"), factors, 0.75);

        assert_eq!(score.market_id().as_str(), "test");
        assert!((score.composite() - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn market_score_from_factors_computes_composite() {
        let factors = ScoreFactors::new(0.2, 0.4, 0.6, 0.8, 1.0);
        let weights = ScoreWeights::default();
        let score = MarketScore::from_factors(make_market_id("test"), factors, &weights);

        // With default weights (all 1.0), composite = average = 0.6
        assert!((score.composite() - 0.6).abs() < f64::EPSILON);
    }

    #[test]
    fn market_score_accessors() {
        let factors = ScoreFactors::new(0.1, 0.2, 0.3, 0.4, 0.5);
        let score = MarketScore::new(make_market_id("market-123"), factors, 0.3);

        assert_eq!(score.market_id().as_str(), "market-123");
        assert!((score.factors().liquidity - 0.1).abs() < f64::EPSILON);
        assert!((score.composite() - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn market_score_ord_higher_composite_is_greater() {
        let factors = ScoreFactors::default();
        let low = MarketScore::new(make_market_id("low"), factors, 0.3);
        let high = MarketScore::new(make_market_id("high"), factors, 0.7);

        assert!(high > low);
        assert!(low < high);
    }

    #[test]
    fn market_score_ord_equal_composites_are_equal() {
        let factors = ScoreFactors::default();
        let a = MarketScore::new(make_market_id("a"), factors, 0.5);
        let b = MarketScore::new(make_market_id("b"), factors, 0.5);

        assert!(a == b);
        assert!(!(a > b));
        assert!(!(a < b));
    }

    #[test]
    fn market_score_sorting_by_priority() {
        let factors = ScoreFactors::default();
        let mut scores = vec![
            MarketScore::new(make_market_id("medium"), factors, 0.5),
            MarketScore::new(make_market_id("low"), factors, 0.2),
            MarketScore::new(make_market_id("high"), factors, 0.8),
        ];

        scores.sort();

        // After sorting, lowest to highest
        assert_eq!(scores[0].market_id().as_str(), "low");
        assert_eq!(scores[1].market_id().as_str(), "medium");
        assert_eq!(scores[2].market_id().as_str(), "high");
    }

    #[test]
    fn market_score_sorting_descending_for_priority_queue() {
        let factors = ScoreFactors::default();
        let mut scores = vec![
            MarketScore::new(make_market_id("medium"), factors, 0.5),
            MarketScore::new(make_market_id("low"), factors, 0.2),
            MarketScore::new(make_market_id("high"), factors, 0.8),
        ];

        scores.sort_by(|a, b| b.cmp(a)); // Descending order

        // Highest priority first
        assert_eq!(scores[0].market_id().as_str(), "high");
        assert_eq!(scores[1].market_id().as_str(), "medium");
        assert_eq!(scores[2].market_id().as_str(), "low");
    }
}
