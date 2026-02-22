//! Market scorer for Polymarket exchange.
//!
//! Implements [`MarketScorer`] to score Polymarket markets for subscription prioritization.

use async_trait::async_trait;

use crate::domain::MarketId;
use crate::runtime::{MarketScore, ScoreFactors, ScoreWeights};
use crate::error::Result;
use crate::runtime::exchange::{MarketInfo, MarketScorer};
use crate::runtime::{OutcomeBonusConfig, PolymarketScoringConfig, ScoringWeightsConfig};

/// Scorer for Polymarket markets.
///
/// Analyzes market characteristics and computes scores used for
/// subscription prioritization in adaptive subscription management.
#[derive(Debug, Clone)]
pub struct PolymarketScorer {
    /// Scoring weights for factor combination.
    weights: ScoreWeights,
    /// Outcome bonus configuration.
    outcome_bonus: OutcomeBonusConfig,
}

impl PolymarketScorer {
    /// Create a new Polymarket scorer from configuration.
    #[must_use]
    pub fn new(config: &PolymarketScoringConfig) -> Self {
        Self {
            weights: Self::weights_from_config(&config.weights),
            outcome_bonus: config.outcome_bonus.clone(),
        }
    }

    /// Convert config weights to domain weights.
    fn weights_from_config(config: &ScoringWeightsConfig) -> ScoreWeights {
        ScoreWeights::new(
            config.liquidity,
            config.spread,
            config.opportunity,
            config.outcome_count,
            config.activity,
        )
    }

    /// Calculate opportunity score based on price imbalance.
    ///
    /// For prediction markets, outcome prices should sum to 1.0. Any deviation
    /// from this indicates potential mispricing and arbitrage opportunity.
    ///
    /// Returns a normalized score (0.0-1.0) where:
    /// - 0.0 = prices sum to exactly 1.0 (no opportunity)
    /// - 1.0 = prices deviate by 10% or more from 1.0 (high opportunity)
    ///
    /// Returns 0.5 (neutral) if price data is unavailable.
    #[must_use]
    pub fn opportunity_score(&self, market: &MarketInfo) -> f64 {
        let prices: Vec<f64> = market.outcomes.iter().filter_map(|o| o.price).collect();

        // No price data or incomplete data - return neutral score
        if prices.is_empty() || prices.len() != market.outcomes.len() {
            return 0.5;
        }

        let sum: f64 = prices.iter().sum();
        let deviation = (sum - 1.0).abs();

        // Higher deviation = higher opportunity
        // Cap at 0.10 (10% mispricing) for normalization
        (deviation / 0.10).min(1.0)
    }

    /// Calculate outcome score based on outcome count.
    ///
    /// Returns a normalized score (0.0-1.0) based on the number of outcomes:
    /// - Binary (2 outcomes): Uses `binary` bonus (default 1.0)
    /// - 3-5 outcomes: Uses `three_to_five` bonus (default 1.5)
    /// - 6+ outcomes: Uses `six_plus` bonus (default 2.0)
    ///
    /// The raw bonus is normalized to the 0.0-1.0 range by dividing by the
    /// maximum possible bonus (six_plus).
    #[must_use]
    pub fn outcome_score(&self, outcome_count: usize) -> f64 {
        let raw_score = match outcome_count {
            0..=2 => self.outcome_bonus.binary,
            3..=5 => self.outcome_bonus.three_to_five,
            _ => self.outcome_bonus.six_plus,
        };

        // Normalize to 0.0-1.0 range using max bonus as normalizer
        let max_bonus = self
            .outcome_bonus
            .binary
            .max(self.outcome_bonus.three_to_five)
            .max(self.outcome_bonus.six_plus);

        if max_bonus == 0.0 {
            0.0
        } else {
            raw_score / max_bonus
        }
    }
}

#[async_trait]
impl MarketScorer for PolymarketScorer {
    async fn score(&self, market: &MarketInfo) -> Result<MarketScore> {
        // Calculate scores from available data
        let outcome_score = self.outcome_score(market.outcomes.len());
        let opportunity = self.opportunity_score(market);

        // Phase 1: Only use factors we can calculate from REST API data
        // Set others to neutral 0.5 (won't affect relative ranking with zero weights)
        let factors = ScoreFactors::new(
            0.5,         // liquidity - needs order book (Phase 2)
            0.5,         // spread - needs order book (Phase 2)
            opportunity, // from price imbalance
            outcome_score,
            0.5, // activity - needs trade data (Phase 2)
        );

        let market_id = MarketId::from(market.id.as_str());
        Ok(MarketScore::from_factors(market_id, factors, &self.weights))
    }

    fn weights(&self) -> &ScoreWeights {
        &self.weights
    }

    fn exchange_name(&self) -> &'static str {
        "polymarket"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::exchange::OutcomeInfo;

    fn default_config() -> PolymarketScoringConfig {
        PolymarketScoringConfig::default()
    }

    fn make_market(id: &str, outcome_count: usize) -> MarketInfo {
        let outcomes: Vec<OutcomeInfo> = (0..outcome_count)
            .map(|i| OutcomeInfo {
                token_id: format!("token-{}", i),
                name: format!("Outcome {}", i),
                price: None,
            })
            .collect();

        MarketInfo {
            id: id.to_string(),
            question: format!("Test market {}", id),
            outcomes,
            active: true,
            volume_24h: None,
            liquidity: None,
        }
    }

    fn make_market_with_prices(id: &str, prices: &[f64]) -> MarketInfo {
        let outcomes: Vec<OutcomeInfo> = prices
            .iter()
            .enumerate()
            .map(|(i, &price)| OutcomeInfo {
                token_id: format!("token-{}", i),
                name: format!("Outcome {}", i),
                price: Some(price),
            })
            .collect();

        MarketInfo {
            id: id.to_string(),
            question: format!("Test market {}", id),
            outcomes,
            active: true,
            volume_24h: None,
            liquidity: None,
        }
    }

    // --- Constructor tests ---

    #[test]
    fn new_creates_scorer_with_config_weights() {
        let config = default_config();
        let scorer = PolymarketScorer::new(&config);

        assert!((scorer.weights.liquidity - config.weights.liquidity).abs() < f64::EPSILON);
        assert!((scorer.weights.spread - config.weights.spread).abs() < f64::EPSILON);
        assert!((scorer.weights.opportunity - config.weights.opportunity).abs() < f64::EPSILON);
        assert!((scorer.weights.outcome_count - config.weights.outcome_count).abs() < f64::EPSILON);
        assert!((scorer.weights.activity - config.weights.activity).abs() < f64::EPSILON);
    }

    #[test]
    fn new_creates_scorer_with_outcome_bonus_config() {
        let config = default_config();
        let scorer = PolymarketScorer::new(&config);

        assert!((scorer.outcome_bonus.binary - config.outcome_bonus.binary).abs() < f64::EPSILON);
        assert!(
            (scorer.outcome_bonus.three_to_five - config.outcome_bonus.three_to_five).abs()
                < f64::EPSILON
        );
        assert!(
            (scorer.outcome_bonus.six_plus - config.outcome_bonus.six_plus).abs() < f64::EPSILON
        );
    }

    // --- Outcome score tests ---

    #[test]
    fn outcome_score_binary_market() {
        let scorer = PolymarketScorer::new(&default_config());

        // Binary market (2 outcomes) uses binary bonus = 1.0
        // Normalized by max (2.0): 1.0 / 2.0 = 0.5
        let score = scorer.outcome_score(2);
        assert!((score - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn outcome_score_zero_outcomes_uses_binary() {
        let scorer = PolymarketScorer::new(&default_config());

        // Edge case: 0 outcomes treated as binary
        let score = scorer.outcome_score(0);
        assert!((score - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn outcome_score_three_outcomes() {
        let scorer = PolymarketScorer::new(&default_config());

        // 3 outcomes uses three_to_five bonus = 1.5
        // Normalized by max (2.0): 1.5 / 2.0 = 0.75
        let score = scorer.outcome_score(3);
        assert!((score - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn outcome_score_five_outcomes() {
        let scorer = PolymarketScorer::new(&default_config());

        // 5 outcomes uses three_to_five bonus = 1.5
        let score = scorer.outcome_score(5);
        assert!((score - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn outcome_score_six_plus_outcomes() {
        let scorer = PolymarketScorer::new(&default_config());

        // 6+ outcomes uses six_plus bonus = 2.0
        // Normalized by max (2.0): 2.0 / 2.0 = 1.0
        let score = scorer.outcome_score(6);
        assert!((score - 1.0).abs() < f64::EPSILON);

        let score = scorer.outcome_score(10);
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn outcome_score_custom_bonus_config() {
        let mut config = default_config();
        config.outcome_bonus.binary = 0.5;
        config.outcome_bonus.three_to_five = 1.0;
        config.outcome_bonus.six_plus = 1.5;

        let scorer = PolymarketScorer::new(&config);

        // Binary normalized: 0.5 / 1.5 = 0.333...
        let binary_score = scorer.outcome_score(2);
        assert!((binary_score - (0.5 / 1.5)).abs() < f64::EPSILON);

        // Three_to_five normalized: 1.0 / 1.5 = 0.666...
        let three_score = scorer.outcome_score(4);
        assert!((three_score - (1.0 / 1.5)).abs() < f64::EPSILON);

        // Six_plus normalized: 1.5 / 1.5 = 1.0
        let six_score = scorer.outcome_score(7);
        assert!((six_score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn outcome_score_zero_max_bonus_returns_zero() {
        let mut config = default_config();
        config.outcome_bonus.binary = 0.0;
        config.outcome_bonus.three_to_five = 0.0;
        config.outcome_bonus.six_plus = 0.0;

        let scorer = PolymarketScorer::new(&config);

        assert!((scorer.outcome_score(2)).abs() < f64::EPSILON);
        assert!((scorer.outcome_score(5)).abs() < f64::EPSILON);
        assert!((scorer.outcome_score(10)).abs() < f64::EPSILON);
    }

    // --- Opportunity score tests ---

    const SCORE_TOLERANCE: f64 = 1e-10;

    #[test]
    fn opportunity_score_balanced_binary_market() {
        let scorer = PolymarketScorer::new(&default_config());

        // Prices sum to 1.0 = no mispricing = score 0.0
        let market = make_market_with_prices("balanced", &[0.5, 0.5]);
        let score = scorer.opportunity_score(&market);
        assert!(score.abs() < SCORE_TOLERANCE);

        // Another balanced case
        let market = make_market_with_prices("balanced2", &[0.7, 0.3]);
        let score = scorer.opportunity_score(&market);
        assert!(score.abs() < SCORE_TOLERANCE);
    }

    #[test]
    fn opportunity_score_moderate_imbalance() {
        let scorer = PolymarketScorer::new(&default_config());

        // Prices sum to 0.95 = 5% deviation = score 0.5
        let market = make_market_with_prices("moderate", &[0.5, 0.45]);
        let score = scorer.opportunity_score(&market);
        assert!((score - 0.5).abs() < SCORE_TOLERANCE);

        // Prices sum to 1.05 = 5% deviation = score 0.5
        let market = make_market_with_prices("moderate2", &[0.55, 0.5]);
        let score = scorer.opportunity_score(&market);
        assert!((score - 0.5).abs() < SCORE_TOLERANCE);
    }

    #[test]
    fn opportunity_score_high_imbalance() {
        let scorer = PolymarketScorer::new(&default_config());

        // Prices sum to 0.90 = 10% deviation = score 1.0 (max)
        let market = make_market_with_prices("high", &[0.5, 0.4]);
        let score = scorer.opportunity_score(&market);
        assert!((score - 1.0).abs() < SCORE_TOLERANCE);

        // Prices sum to 0.85 = 15% deviation = capped at 1.0
        let market = make_market_with_prices("very_high", &[0.5, 0.35]);
        let score = scorer.opportunity_score(&market);
        assert!((score - 1.0).abs() < SCORE_TOLERANCE);
    }

    #[test]
    fn opportunity_score_missing_prices_returns_neutral() {
        let scorer = PolymarketScorer::new(&default_config());

        // No prices at all
        let market = make_market("no_prices", 2);
        let score = scorer.opportunity_score(&market);
        assert!((score - 0.5).abs() < SCORE_TOLERANCE);
    }

    #[test]
    fn opportunity_score_partial_prices_returns_neutral() {
        let scorer = PolymarketScorer::new(&default_config());

        // Only some outcomes have prices
        let market = MarketInfo {
            id: "partial".to_string(),
            question: "Partial prices".to_string(),
            outcomes: vec![
                OutcomeInfo {
                    token_id: "t0".to_string(),
                    name: "Yes".to_string(),
                    price: Some(0.5),
                },
                OutcomeInfo {
                    token_id: "t1".to_string(),
                    name: "No".to_string(),
                    price: None, // Missing!
                },
            ],
            active: true,
            volume_24h: None,
            liquidity: None,
        };
        let score = scorer.opportunity_score(&market);
        assert!((score - 0.5).abs() < SCORE_TOLERANCE);
    }

    #[test]
    fn opportunity_score_multi_outcome_market() {
        let scorer = PolymarketScorer::new(&default_config());

        // Multi-outcome balanced: prices sum to 1.0
        let market = make_market_with_prices("multi_balanced", &[0.25, 0.25, 0.25, 0.25]);
        let score = scorer.opportunity_score(&market);
        assert!(score.abs() < SCORE_TOLERANCE);

        // Multi-outcome imbalanced: prices sum to 0.92 = 8% deviation = score 0.8
        let market = make_market_with_prices("multi_imbalanced", &[0.20, 0.20, 0.20, 0.32]);
        let score = scorer.opportunity_score(&market);
        assert!((score - 0.8).abs() < SCORE_TOLERANCE);
    }

    // --- MarketScorer trait tests ---

    #[tokio::test]
    async fn score_returns_market_score_with_correct_id() {
        let scorer = PolymarketScorer::new(&default_config());
        let market = make_market("test-market-123", 2);

        let score = scorer.score(&market).await.unwrap();

        assert_eq!(score.market_id().as_str(), "test-market-123");
    }

    #[tokio::test]
    async fn score_uses_neutral_values_when_data_unavailable() {
        let scorer = PolymarketScorer::new(&default_config());
        let market = make_market("test", 2); // No price data

        let score = scorer.score(&market).await.unwrap();
        let factors = score.factors();

        // Phase 1: liquidity, spread, activity are neutral (need order book data)
        // opportunity is also neutral when price data is missing
        assert!((factors.liquidity - 0.5).abs() < f64::EPSILON);
        assert!((factors.spread - 0.5).abs() < f64::EPSILON);
        assert!((factors.opportunity - 0.5).abs() < f64::EPSILON);
        assert!((factors.activity - 0.5).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn score_calculates_outcome_factor_from_market() {
        let scorer = PolymarketScorer::new(&default_config());

        // Binary market
        let binary_market = make_market("binary", 2);
        let binary_score = scorer.score(&binary_market).await.unwrap();
        assert!((binary_score.factors().outcome_count - 0.5).abs() < f64::EPSILON);

        // Multi-outcome market
        let multi_market = make_market("multi", 6);
        let multi_score = scorer.score(&multi_market).await.unwrap();
        assert!((multi_score.factors().outcome_count - 1.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn score_computes_composite_with_weights() {
        let mut config = default_config();
        // Set all weights equal for easier calculation
        config.weights.liquidity = 1.0;
        config.weights.spread = 1.0;
        config.weights.opportunity = 1.0;
        config.weights.outcome_count = 1.0;
        config.weights.activity = 1.0;

        let scorer = PolymarketScorer::new(&config);
        let market = make_market("test", 2); // No price data, binary market

        let score = scorer.score(&market).await.unwrap();

        // All factors are 0.5:
        // - liquidity, spread, activity: neutral (Phase 2)
        // - opportunity: neutral (no price data)
        // - outcome_count: binary market = 0.5
        // Composite: (0.5 + 0.5 + 0.5 + 0.5 + 0.5) / 5 = 0.5
        assert!((score.composite() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn weights_returns_configured_weights() {
        let mut config = default_config();
        config.weights.liquidity = 0.3;
        config.weights.spread = 0.2;
        config.weights.opportunity = 0.15;
        config.weights.outcome_count = 0.1;
        config.weights.activity = 0.25;

        let scorer = PolymarketScorer::new(&config);
        let weights = scorer.weights();

        assert!((weights.liquidity - 0.3).abs() < f64::EPSILON);
        assert!((weights.spread - 0.2).abs() < f64::EPSILON);
        assert!((weights.opportunity - 0.15).abs() < f64::EPSILON);
        assert!((weights.outcome_count - 0.1).abs() < f64::EPSILON);
        assert!((weights.activity - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn exchange_name_returns_polymarket() {
        let scorer = PolymarketScorer::new(&default_config());

        assert_eq!(scorer.exchange_name(), "polymarket");
    }

    // --- Batch scoring tests ---

    #[tokio::test]
    async fn score_batch_scores_multiple_markets() {
        let scorer = PolymarketScorer::new(&default_config());
        let markets = vec![
            make_market("market-1", 2),
            make_market("market-2", 4),
            make_market("market-3", 8),
        ];

        let scores = scorer.score_batch(&markets).await.unwrap();

        assert_eq!(scores.len(), 3);
        assert_eq!(scores[0].market_id().as_str(), "market-1");
        assert_eq!(scores[1].market_id().as_str(), "market-2");
        assert_eq!(scores[2].market_id().as_str(), "market-3");
    }

    #[tokio::test]
    async fn score_batch_empty_returns_empty() {
        let scorer = PolymarketScorer::new(&default_config());
        let markets: Vec<MarketInfo> = vec![];

        let scores = scorer.score_batch(&markets).await.unwrap();

        assert!(scores.is_empty());
    }
}
