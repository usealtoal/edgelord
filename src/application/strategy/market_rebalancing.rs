//! Market rebalancing arbitrage strategy.
//!
//! Detects when the sum of all outcome ask prices is less than the guaranteed
//! payout in multi-outcome markets (3+ outcomes).
//!
//! Historical data shows this strategy captured 73.1% ($29M) of arbitrage
//! profits, making it the largest contributor by far.

use rust_decimal::Decimal;
use serde::Deserialize;

use crate::domain::{
    id::MarketId, id::TokenId, money::Price, money::Volume, opportunity::Opportunity,
    opportunity::OpportunityLeg,
};
use crate::port::{
    inbound::strategy::DetectionContext, inbound::strategy::MarketContext,
    inbound::strategy::Strategy,
};

/// Configuration for market rebalancing arbitrage detection.
#[derive(Debug, Clone, Deserialize)]
pub struct MarketRebalancingConfig {
    /// Minimum edge (profit per dollar) required to consider an opportunity.
    #[serde(default = "default_min_edge")]
    pub min_edge: Decimal,

    /// Minimum expected profit in dollars required to execute.
    #[serde(default = "default_min_profit")]
    pub min_profit: Decimal,

    /// Maximum number of outcomes to analyze.
    /// Markets with more outcomes are skipped to avoid performance issues.
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

/// Market rebalancing arbitrage detector for multi-outcome markets.
///
/// Identifies opportunities where purchasing all outcomes costs less than
/// the guaranteed payout. Since exactly one outcome must win, the position
/// guarantees a profit equal to (payout - total cost).
///
/// Binary markets (2 outcomes) are handled by
/// [`SingleConditionStrategy`](super::single_condition::SingleConditionStrategy)
/// instead for efficiency.
pub struct MarketRebalancingStrategy {
    /// Strategy configuration.
    config: MarketRebalancingConfig,
}

impl MarketRebalancingStrategy {
    /// Create a new strategy with the given configuration.
    #[must_use]
    pub const fn new(config: MarketRebalancingConfig) -> Self {
        Self { config }
    }

    /// Return the current configuration.
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

    fn detect(&self, ctx: &dyn DetectionContext) -> Vec<Opportunity> {
        let market = ctx.market();

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

        let payout = ctx.payout();

        // Use the detection function
        if let Some(rebal_opp) = detect_rebalancing(ctx, &token_ids, &self.config, payout) {
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
    /// Token identifier for this outcome.
    pub token_id: TokenId,
    /// Best ask price for this outcome.
    pub price: Price,
    /// Available volume at the ask price.
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

/// A market rebalancing opportunity with multiple outcome legs.
///
/// Unlike single-condition which has exactly 2 legs (YES/NO), rebalancing
/// opportunities can have many legs (one per outcome in the market).
#[derive(Debug, Clone)]
pub struct RebalancingOpportunity {
    /// Identifier of the market.
    pub market_id: MarketId,
    /// Market question text.
    pub question: String,
    /// All legs in the opportunity (one per outcome).
    pub legs: Vec<RebalancingLeg>,
    /// Total cost to purchase all outcomes.
    pub total_cost: Price,
    /// Edge (profit per dollar of payout).
    pub edge: Price,
    /// Tradeable volume, limited by the smallest leg.
    pub volume: Volume,
    /// Expected profit at the tradeable volume.
    pub expected_profit: Price,
}

impl RebalancingOpportunity {
    /// Return the number of outcomes in this opportunity.
    #[must_use]
    pub fn outcome_count(&self) -> usize {
        self.legs.len()
    }
}

/// Detect a rebalancing opportunity across multiple outcomes.
///
/// Returns `None` if:
/// - The market has fewer than 3 or more than `max_outcomes` outcomes
/// - Any required order book is missing or has no asks
/// - The total cost equals or exceeds the payout
/// - The edge is below the configured minimum
/// - The expected profit is below the configured minimum
pub fn detect_rebalancing(
    ctx: &dyn DetectionContext,
    token_ids: &[TokenId],
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

    // Fail closed if any required order book is missing
    for token_id in token_ids {
        let book = ctx.order_book(token_id)?;
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
        market_id: ctx.market_id().clone(),
        question: ctx.question().to_string(),
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
    use crate::application::cache::book::BookCache;
    use crate::application::strategy::context::ConcreteDetectionContext;
    use crate::domain::{book::Book, book::PriceLevel, market::Market, market::Outcome};
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
        let tokens = vec![
            TokenId::from("candidate-a"),
            TokenId::from("candidate-b"),
            TokenId::from("candidate-c"),
        ];
        let outcomes: Vec<Outcome> = tokens
            .iter()
            .enumerate()
            .map(|(i, t)| {
                Outcome::new(t.clone(), format!("Candidate {}", (b'A' + i as u8) as char))
            })
            .collect();
        let market = Market::new(MarketId::from("election"), "Who wins?", outcomes, dec!(1));

        let cache = BookCache::new();
        let config = make_config();

        // 0.30 + 0.30 + 0.30 = 0.90 (10% edge)
        cache.update(Book::with_levels(
            tokens[0].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.30), dec!(100))],
        ));
        cache.update(Book::with_levels(
            tokens[1].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.30), dec!(100))],
        ));
        cache.update(Book::with_levels(
            tokens[2].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.30), dec!(100))],
        ));

        let ctx = ConcreteDetectionContext::new(&market, &cache);
        let opp = detect_rebalancing(&ctx, &tokens, &config, Decimal::ONE);
        assert!(opp.is_some());

        let opp = opp.unwrap();
        assert_eq!(opp.total_cost, dec!(0.90));
        assert_eq!(opp.edge, dec!(0.10));
        assert_eq!(opp.expected_profit, dec!(10.00));
        assert_eq!(opp.legs.len(), 3);
    }

    #[test]
    fn test_no_opportunity_when_sum_exceeds_one() {
        let tokens = vec![
            TokenId::from("candidate-a"),
            TokenId::from("candidate-b"),
            TokenId::from("candidate-c"),
        ];
        let outcomes: Vec<Outcome> = tokens
            .iter()
            .enumerate()
            .map(|(i, t)| {
                Outcome::new(t.clone(), format!("Candidate {}", (b'A' + i as u8) as char))
            })
            .collect();
        let market = Market::new(MarketId::from("election"), "Who wins?", outcomes, dec!(1));

        let cache = BookCache::new();
        let config = make_config();

        // 0.40 + 0.40 + 0.40 = 1.20 (no edge)
        cache.update(Book::with_levels(
            tokens[0].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.40), dec!(100))],
        ));
        cache.update(Book::with_levels(
            tokens[1].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.40), dec!(100))],
        ));
        cache.update(Book::with_levels(
            tokens[2].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.40), dec!(100))],
        ));

        let ctx = ConcreteDetectionContext::new(&market, &cache);
        assert!(detect_rebalancing(&ctx, &tokens, &config, Decimal::ONE).is_none());
    }

    #[test]
    fn test_volume_limited_by_smallest_leg() {
        let tokens = vec![TokenId::from("a"), TokenId::from("b"), TokenId::from("c")];
        let outcomes: Vec<Outcome> = tokens
            .iter()
            .enumerate()
            .map(|(i, t)| Outcome::new(t.clone(), format!("Option {}", (b'A' + i as u8) as char)))
            .collect();
        let market = Market::new(MarketId::from("election"), "Who wins?", outcomes, dec!(1));

        let cache = BookCache::new();
        let config = make_config();

        // Different volumes: 50, 100, 200 -> limited to 50
        cache.update(Book::with_levels(
            tokens[0].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.30), dec!(50))],
        ));
        cache.update(Book::with_levels(
            tokens[1].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.30), dec!(100))],
        ));
        cache.update(Book::with_levels(
            tokens[2].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.30), dec!(200))],
        ));

        let ctx = ConcreteDetectionContext::new(&market, &cache);
        let opp = detect_rebalancing(&ctx, &tokens, &config, Decimal::ONE).unwrap();
        assert_eq!(opp.volume, dec!(50));
        assert_eq!(opp.expected_profit, dec!(5.00)); // 50 * 0.10
    }

    #[test]
    fn test_rejects_binary_markets() {
        let tokens = vec![TokenId::from("yes"), TokenId::from("no")];
        let outcomes: Vec<Outcome> = vec![
            Outcome::new(tokens[0].clone(), "Yes"),
            Outcome::new(tokens[1].clone(), "No"),
        ];
        let market = Market::new(MarketId::from("binary"), "Yes/No?", outcomes, dec!(1));

        let cache = BookCache::new();
        let config = make_config();

        cache.update(Book::with_levels(
            tokens[0].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.40), dec!(100))],
        ));
        cache.update(Book::with_levels(
            tokens[1].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(100))],
        ));

        let ctx = ConcreteDetectionContext::new(&market, &cache);
        // Should return None for binary markets (handled by single_condition)
        assert!(detect_rebalancing(&ctx, &tokens, &config, Decimal::ONE).is_none());
    }
}
