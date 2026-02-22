//! Market scoring types for subscription prioritization.
//!
//! These types are used to score and prioritize markets for subscription management.
//! Markets with higher scores should be prioritized for active subscriptions.

use std::cmp::Ordering;

use super::id::MarketId;

/// Individual scoring factors for a market.
///
/// Each factor is normalized to the 0.0-1.0 range where higher values indicate
/// more desirable characteristics for subscription prioritization.
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
    fn cmp(&self, other: &Self) -> Ordering {
        self.composite
            .partial_cmp(&other.composite)
            .unwrap_or(Ordering::Equal)
    }
}
