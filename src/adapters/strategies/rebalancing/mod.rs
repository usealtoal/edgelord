//! Market rebalancing arbitrage strategy.
//!
//! Detects when the sum of all outcome prices < $1.00 in multi-outcome markets.
//! This captured 73.1% ($29M) of historical arbitrage profits - the largest share!

use rust_decimal::Decimal;
use serde::Deserialize;

use super::{DetectionContext, MarketContext};
use crate::core::strategy::Strategy;
use crate::core::cache::OrderBookCache;
use crate::core::domain::{MarketId, Opportunity, OpportunityLeg, Price, TokenId, Volume};

/// Configuration for market rebalancing detection.
#[derive(Debug, Clone, Deserialize)]
pub struct MarketRebalancingConfig {
    /// Minimum edge (profit per $1) to consider.
    #[serde(default = "default_min_edge")]
    pub min_edge: Decimal,

    /// Minimum expected profit in dollars.
    #[serde(default = "default_min_profit")]
    pub min_profit: Decimal,

    /// Maximum number of outcomes to analyze (skip huge markets).
    #[serde(default = "default_max_outcomes")]
    pub max_outcomes: usize,
}

fn default_min_edge() -> Decimal {
    Decimal::new(3, 2) // 0.03 (accounts for gas costs)
}

fn default_min_profit() -> Decimal {
    Decimal::new(25, 2) // $0.25
}

const fn default_max_outcomes() -> usize {
    10
}

impl Default for MarketRebalancingConfig {
    fn default() -> Self {
        Self {
            min_edge: default_min_edge(),
            min_profit: default_min_profit(),
            max_outcomes: default_max_outcomes(),
        }
    }
}

/// Market rebalancing arbitrage detector.
///
/// Finds opportunities where buying all outcomes costs less than $1.00.
/// Since exactly one outcome must win, guaranteed $1.00 payout.
pub struct MarketRebalancingStrategy {
    config: MarketRebalancingConfig,
}

impl MarketRebalancingStrategy {
    /// Create a new strategy with the given configuration.
    #[must_use]
    pub const fn new(config: MarketRebalancingConfig) -> Self {
        Self { config }
    }

    /// Get the strategy configuration.
    #[must_use]
    pub const fn config(&self) -> &MarketRebalancingConfig {
        &self.config
    }
}

impl Strategy for MarketRebalancingStrategy {
    fn name(&self) -> &'static str {
        "market_rebalancing"
    }

    fn applies_to(&self, ctx: &MarketContext) -> bool {
        // Applies to multi-outcome markets (3+ outcomes)
        // Binary markets are handled more efficiently by single_condition
        ctx.is_multi_outcome() && ctx.outcome_count <= self.config.max_outcomes
    }

    fn detect(&self, ctx: &DetectionContext) -> Vec<Opportunity> {
        let market = ctx.market;

        // Need at least 3 outcomes for rebalancing (binary handled by single_condition)
        if market.outcome_count() < 3 {
            return vec![];
        }

        // Collect token IDs from market outcomes
        let token_ids: Vec<TokenId> = market
            .outcomes()
            .iter()
            .map(|o| o.token_id().clone())
            .collect();

        let payout = market.payout();

        // Use the existing detection function
        if let Some(rebal_opp) = detect_rebalancing(
            market.market_id(),
            market.question(),
            &token_ids,
            ctx.cache,
            &self.config,
            payout,
        ) {
            // Convert RebalancingOpportunity to standard Opportunity
            let legs: Vec<OpportunityLeg> = rebal_opp
                .legs
                .iter()
                .map(|leg| OpportunityLeg::new(leg.token_id.clone(), leg.price))
                .collect();

            let opp = Opportunity::with_strategy(
                rebal_opp.market_id.clone(),
                &rebal_opp.question,
                legs,
                rebal_opp.volume,
                payout,
                "market_rebalancing",
            );
            return vec![opp];
        }

        vec![]
    }
}

/// A single leg in a rebalancing opportunity.
#[derive(Debug, Clone)]
pub struct RebalancingLeg {
    /// Token ID for this outcome.
    pub token_id: TokenId,
    /// Ask price for this outcome.
    pub price: Price,
    /// Available volume at this price.
    pub volume: Volume,
}

impl RebalancingLeg {
    /// Create a new rebalancing leg.
    #[must_use]
    pub const fn new(token_id: TokenId, price: Price, volume: Volume) -> Self {
        Self {
            token_id,
            price,
            volume,
        }
    }
}

/// A market rebalancing opportunity.
///
/// Unlike single-condition which only has YES/NO, rebalancing
/// can have many legs (one per outcome).
#[derive(Debug, Clone)]
pub struct RebalancingOpportunity {
    /// Market ID.
    pub market_id: MarketId,
    /// Market question.
    pub question: String,
    /// All legs (one per outcome).
    pub legs: Vec<RebalancingLeg>,
    /// Total cost to buy all outcomes.
    pub total_cost: Price,
    /// Edge (profit per $1).
    pub edge: Price,
    /// Tradeable volume (limited by smallest leg).
    pub volume: Volume,
    /// Expected profit.
    pub expected_profit: Price,
}

impl RebalancingOpportunity {
    /// Number of outcomes in this opportunity.
    #[must_use]
    pub const fn outcome_count(&self) -> usize {
        self.legs.len()
    }
}

/// Detect rebalancing opportunity across multiple outcomes.
///
/// # Arguments
/// * `market_id` - Market identifier
/// * `question` - Market question/description
/// * `token_ids` - All outcome token IDs for the market
/// * `cache` - Order book cache with current prices
/// * `config` - Detection thresholds
/// * `payout` - The payout amount for the market
///
/// # Returns
/// `Some(RebalancingOpportunity)` if sum of best asks < payout
pub fn detect_rebalancing(
    market_id: &MarketId,
    question: &str,
    token_ids: &[TokenId],
    cache: &OrderBookCache,
    config: &MarketRebalancingConfig,
    payout: Decimal,
) -> Option<RebalancingOpportunity> {
    // Need at least 3 outcomes (2 is handled by single_condition)
    if token_ids.len() < 3 || token_ids.len() > config.max_outcomes {
        return None;
    }

    // Collect best asks for all outcomes
    let mut legs = Vec::with_capacity(token_ids.len());
    let mut total_cost = Decimal::ZERO;
    let mut min_volume = Decimal::MAX;

    // Fail closed if any required order book is missing.
    for token_id in token_ids {
        let book = cache.get(token_id)?;
        let ask = book.best_ask()?;

        total_cost += ask.price();
        min_volume = min_volume.min(ask.size());

        legs.push(RebalancingLeg::new(
            token_id.clone(),
            ask.price(),
            ask.size(),
        ));
    }

    // Check if arbitrage exists
    if total_cost >= payout {
        return None;
    }

    let edge = payout - total_cost;

    if edge < config.min_edge {
        return None;
    }

    let expected_profit = edge * min_volume;

    if expected_profit < config.min_profit {
        return None;
    }

    Some(RebalancingOpportunity {
        market_id: market_id.clone(),
        question: question.to_string(),
        legs,
        total_cost,
        edge,
        volume: min_volume,
        expected_profit,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::domain::{OrderBook, PriceLevel};
    use rust_decimal_macros::dec;

    fn make_config() -> MarketRebalancingConfig {
        MarketRebalancingConfig {
            min_edge: dec!(0.03),
            min_profit: dec!(1.00),
            max_outcomes: 10,
        }
    }

    #[test]
    fn test_strategy_name() {
        let strategy = MarketRebalancingStrategy::new(make_config());
        assert_eq!(strategy.name(), "market_rebalancing");
    }

    #[test]
    fn test_applies_to_multi_outcome_only() {
        let strategy = MarketRebalancingStrategy::new(make_config());

        assert!(!strategy.applies_to(&MarketContext::binary()));
        assert!(strategy.applies_to(&MarketContext::multi_outcome(3)));
        assert!(strategy.applies_to(&MarketContext::multi_outcome(5)));
        assert!(strategy.applies_to(&MarketContext::multi_outcome(10)));
        assert!(!strategy.applies_to(&MarketContext::multi_outcome(11))); // exceeds max
    }

    #[test]
    fn test_detect_rebalancing_opportunity() {
        let market_id = MarketId::from("election");
        let tokens = vec![
            TokenId::from("candidate-a"),
            TokenId::from("candidate-b"),
            TokenId::from("candidate-c"),
        ];
        let cache = OrderBookCache::new();
        let config = make_config();

        // 0.30 + 0.30 + 0.30 = 0.90 (10% edge)
        cache.update(OrderBook::with_levels(
            tokens[0].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.30), dec!(100))],
        ));
        cache.update(OrderBook::with_levels(
            tokens[1].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.30), dec!(100))],
        ));
        cache.update(OrderBook::with_levels(
            tokens[2].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.30), dec!(100))],
        ));

        let opp = detect_rebalancing(
            &market_id,
            "Who wins?",
            &tokens,
            &cache,
            &config,
            Decimal::ONE,
        );
        assert!(opp.is_some());

        let opp = opp.unwrap();
        assert_eq!(opp.total_cost, dec!(0.90));
        assert_eq!(opp.edge, dec!(0.10));
        assert_eq!(opp.expected_profit, dec!(10.00));
        assert_eq!(opp.legs.len(), 3);
    }

    #[test]
    fn test_no_opportunity_when_sum_exceeds_one() {
        let market_id = MarketId::from("election");
        let tokens = vec![
            TokenId::from("candidate-a"),
            TokenId::from("candidate-b"),
            TokenId::from("candidate-c"),
        ];
        let cache = OrderBookCache::new();
        let config = make_config();

        // 0.40 + 0.40 + 0.40 = 1.20 (no edge)
        cache.update(OrderBook::with_levels(
            tokens[0].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.40), dec!(100))],
        ));
        cache.update(OrderBook::with_levels(
            tokens[1].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.40), dec!(100))],
        ));
        cache.update(OrderBook::with_levels(
            tokens[2].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.40), dec!(100))],
        ));

        assert!(detect_rebalancing(
            &market_id,
            "Who wins?",
            &tokens,
            &cache,
            &config,
            Decimal::ONE
        )
        .is_none());
    }

    #[test]
    fn test_no_opportunity_when_edge_too_small() {
        let market_id = MarketId::from("election");
        let tokens = vec![TokenId::from("a"), TokenId::from("b"), TokenId::from("c")];
        let cache = OrderBookCache::new();
        let config = make_config();

        // 0.33 + 0.33 + 0.33 = 0.99 (only 1% edge, below 3% threshold)
        cache.update(OrderBook::with_levels(
            tokens[0].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.33), dec!(100))],
        ));
        cache.update(OrderBook::with_levels(
            tokens[1].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.33), dec!(100))],
        ));
        cache.update(OrderBook::with_levels(
            tokens[2].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.33), dec!(100))],
        ));

        assert!(detect_rebalancing(
            &market_id,
            "Who wins?",
            &tokens,
            &cache,
            &config,
            Decimal::ONE
        )
        .is_none());
    }

    #[test]
    fn test_volume_limited_by_smallest_leg() {
        let market_id = MarketId::from("election");
        let tokens = vec![TokenId::from("a"), TokenId::from("b"), TokenId::from("c")];
        let cache = OrderBookCache::new();
        let config = make_config();

        // Different volumes: 50, 100, 200 -> limited to 50
        cache.update(OrderBook::with_levels(
            tokens[0].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.30), dec!(50))],
        ));
        cache.update(OrderBook::with_levels(
            tokens[1].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.30), dec!(100))],
        ));
        cache.update(OrderBook::with_levels(
            tokens[2].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.30), dec!(200))],
        ));

        let opp = detect_rebalancing(
            &market_id,
            "Who wins?",
            &tokens,
            &cache,
            &config,
            Decimal::ONE,
        )
        .unwrap();
        assert_eq!(opp.volume, dec!(50));
        assert_eq!(opp.expected_profit, dec!(5.00)); // 50 * 0.10
    }

    #[test]
    fn test_rejects_binary_markets() {
        let market_id = MarketId::from("binary");
        let tokens = vec![TokenId::from("yes"), TokenId::from("no")];
        let cache = OrderBookCache::new();
        let config = make_config();

        cache.update(OrderBook::with_levels(
            tokens[0].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.40), dec!(100))],
        ));
        cache.update(OrderBook::with_levels(
            tokens[1].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(100))],
        ));

        // Should return None for binary markets (handled by single_condition)
        assert!(detect_rebalancing(
            &market_id,
            "Yes/No?",
            &tokens,
            &cache,
            &config,
            Decimal::ONE
        )
        .is_none());
    }

    #[test]
    fn test_custom_payout_affects_edge_calculation() {
        use crate::core::domain::{Market, Outcome};

        // With payout of $100, cost of $90 gives $10 edge (10%)
        // This should be profitable with custom payout
        let strategy = MarketRebalancingStrategy::new(MarketRebalancingConfig {
            min_edge: dec!(5.00), // $5 minimum edge
            min_profit: dec!(1.00),
            max_outcomes: 10,
        });

        let tokens = [
            TokenId::from("candidate-a"),
            TokenId::from("candidate-b"),
            TokenId::from("candidate-c"),
        ];

        // Create market with $1 payout
        let outcomes_1 = vec![
            Outcome::new(tokens[0].clone(), "Candidate A"),
            Outcome::new(tokens[1].clone(), "Candidate B"),
            Outcome::new(tokens[2].clone(), "Candidate C"),
        ];
        let market_1 = Market::new(MarketId::from("election"), "Who wins?", outcomes_1, dec!(1));

        let cache = OrderBookCache::new();

        // Total cost = 30 + 30 + 30 = 90
        // With $1 payout: edge = -$89 (no arbitrage)
        // With $100 payout: edge = $10 (arbitrage exists!)
        cache.update(OrderBook::with_levels(
            tokens[0].clone(),
            vec![],
            vec![PriceLevel::new(dec!(30), dec!(100))],
        ));
        cache.update(OrderBook::with_levels(
            tokens[1].clone(),
            vec![],
            vec![PriceLevel::new(dec!(30), dec!(100))],
        ));
        cache.update(OrderBook::with_levels(
            tokens[2].clone(),
            vec![],
            vec![PriceLevel::new(dec!(30), dec!(100))],
        ));

        // With default $1 payout, no opportunity (cost $90 > payout $1)
        let ctx_default = DetectionContext::new(&market_1, &cache);
        let opps_default = strategy.detect(&ctx_default);
        assert!(
            opps_default.is_empty(),
            "Should have no opportunity with $1 payout"
        );

        // Create market with $100 payout
        let outcomes_100 = vec![
            Outcome::new(tokens[0].clone(), "Candidate A"),
            Outcome::new(tokens[1].clone(), "Candidate B"),
            Outcome::new(tokens[2].clone(), "Candidate C"),
        ];
        let market_100 = Market::new(
            MarketId::from("election"),
            "Who wins?",
            outcomes_100,
            dec!(100),
        );

        // With $100 payout, opportunity exists (cost $90 < payout $100)
        let ctx_custom = DetectionContext::new(&market_100, &cache);
        let opps_custom = strategy.detect(&ctx_custom);
        assert_eq!(
            opps_custom.len(),
            1,
            "Should have opportunity with $100 payout"
        );

        let opp = &opps_custom[0];
        // Edge = payout - total_cost = 100 - 90 = 10
        assert_eq!(opp.edge(), dec!(10));
        // Payout should be $100
        assert_eq!(opp.payout(), dec!(100));
    }
}
