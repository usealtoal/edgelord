//! Market scoring types for subscription prioritization.
//!
//! This module provides types for scoring and ranking markets to prioritize
//! which markets should receive active order book subscriptions.
//!
//! # Scoring System
//!
//! Markets are scored on multiple factors:
//! - **Liquidity**: Market depth and available volume
//! - **Spread**: Tightness of bid-ask spread (tighter is better)
//! - **Opportunity**: Historical arbitrage opportunity frequency
//! - **Outcome Count**: Number of outcomes (more outcomes, more complexity)
//! - **Activity**: Recent trading volume
//!
//! Factors are normalized to 0.0-1.0 and combined using configurable weights.
//!
//! # Examples
//!
//! Scoring a market:
//!
//! ```
//! use edgelord::domain::score::{ScoreFactors, ScoreWeights, MarketScore};
//! use edgelord::domain::id::MarketId;
//!
//! let factors = ScoreFactors::new(
//!     0.8,  // liquidity
//!     0.9,  // spread
//!     0.3,  // opportunity
//!     0.5,  // outcome_count
//!     0.7,  // activity
//! );
//!
//! let weights = ScoreWeights::default();
//! let score = MarketScore::from_factors(
//!     MarketId::new("market-1"),
//!     factors,
//!     &weights,
//! );
//!
//! println!("Composite score: {}", score.composite());
//! ```

use std::cmp::Ordering;

use super::id::MarketId;

/// Individual scoring factors for a market.
///
/// Each factor is normalized to the 0.0-1.0 range where higher values indicate
/// more desirable characteristics for subscription prioritization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScoreFactors {
    /// Market depth and available volume (0.0 to 1.0).
    pub liquidity: f64,
    /// Tightness of bid-ask spread (0.0 to 1.0, higher means tighter).
    pub spread: f64,
    /// Historical arbitrage opportunity frequency (0.0 to 1.0).
    pub opportunity: f64,
    /// Normalized outcome count factor (0.0 to 1.0).
    pub outcome_count: f64,
    /// Recent trading activity level (0.0 to 1.0).
    pub activity: f64,
}

impl ScoreFactors {
    /// Creates new score factors with the given values.
    ///
    /// All values should be normalized to the 0.0-1.0 range.
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

    /// Computes a weighted composite score from these factors.
    ///
    /// Returns the weighted average of all factors.
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
/// Higher weights increase the influence of the corresponding factor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScoreWeights {
    /// Weight applied to the liquidity factor.
    pub liquidity: f64,
    /// Weight applied to the spread factor.
    pub spread: f64,
    /// Weight applied to the opportunity factor.
    pub opportunity: f64,
    /// Weight applied to the outcome count factor.
    pub outcome_count: f64,
    /// Weight applied to the activity factor.
    pub activity: f64,
}

impl ScoreWeights {
    /// Creates new score weights.
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
/// Implements `Ord` for sorting markets by composite score.
#[derive(Debug, Clone)]
pub struct MarketScore {
    /// The scored market's ID.
    market_id: MarketId,
    /// Individual factor scores.
    factors: ScoreFactors,
    /// Combined composite score.
    composite: f64,
}

impl MarketScore {
    /// Creates a new market score with a pre-computed composite.
    #[must_use]
    pub fn new(market_id: MarketId, factors: ScoreFactors, composite: f64) -> Self {
        Self {
            market_id,
            factors,
            composite,
        }
    }

    /// Creates a market score by computing the composite from factors and weights.
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

    /// Returns the market ID.
    #[must_use]
    pub fn market_id(&self) -> &MarketId {
        &self.market_id
    }

    /// Returns the individual score factors.
    #[must_use]
    pub const fn factors(&self) -> &ScoreFactors {
        &self.factors
    }

    /// Returns the composite score.
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
