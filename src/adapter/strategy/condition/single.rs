//! Single-condition arbitrage strategy.
//!
//! Detects when YES + NO < $1.00 for binary markets.
//! This captured 26.7% ($10.5M) of historical arbitrage profits.

use rust_decimal::Decimal;
use serde::Deserialize;

use crate::domain::Opportunity;
use crate::port::{DetectionContext, MarketContext, Strategy};

/// Configuration for single-condition detection.
#[derive(Debug, Clone, Deserialize)]
pub struct SingleConditionConfig {
    /// Minimum edge (profit per $1) to consider.
    #[serde(default = "default_min_edge")]
    pub min_edge: Decimal,

    /// Minimum expected profit in dollars.
    #[serde(default = "default_min_profit")]
    pub min_profit: Decimal,
}

fn default_min_edge() -> Decimal {
    Decimal::new(3, 2) // 0.03 (accounts for gas costs)
}

fn default_min_profit() -> Decimal {
    Decimal::new(10, 2) // 0.10
}

impl Default for SingleConditionConfig {
    fn default() -> Self {
        Self {
            min_edge: default_min_edge(),
            min_profit: default_min_profit(),
        }
    }
}

/// Single-condition arbitrage detector.
///
/// Finds opportunities where buying YES + NO costs less than $1.00.
/// Since one must pay out, guaranteed $1.00 return.
pub struct SingleConditionStrategy {
    config: SingleConditionConfig,
}

impl SingleConditionStrategy {
    /// Create a new strategy with the given configuration.
    #[must_use]
    pub const fn new(config: SingleConditionConfig) -> Self {
        Self { config }
    }

    /// Get the strategy configuration.
    #[must_use]
    pub const fn config(&self) -> &SingleConditionConfig {
        &self.config
    }
}

impl Strategy for SingleConditionStrategy {
    fn name(&self) -> &'static str {
        "single_condition"
    }

    fn applies_to(&self, ctx: &MarketContext) -> bool {
        // Only applies to binary markets (2 outcomes)
        ctx.is_binary()
    }

    fn detect(&self, ctx: &dyn DetectionContext) -> Vec<Opportunity> {
        detect_single_condition(ctx, &self.config)
            .into_iter()
            .collect()
    }
}

/// Core detection logic for single-condition arbitrage.
///
/// Checks if YES ask + NO ask < payout, indicating risk-free profit.
pub fn detect_single_condition(
    ctx: &dyn DetectionContext,
    config: &SingleConditionConfig,
) -> Option<Opportunity> {
    let market = ctx.market();
    let outcomes = market.outcomes();

    // Binary markets have exactly 2 outcomes
    if outcomes.len() != 2 {
        return None;
    }

    let positive_outcome = &outcomes[0];
    let negative_outcome = &outcomes[1];

    // Get order books and best asks
    let positive_book = ctx.order_book(positive_outcome.token_id())?;
    let negative_book = ctx.order_book(negative_outcome.token_id())?;

    let positive_ask = positive_book.best_ask()?;
    let negative_ask = negative_book.best_ask()?;

    let total_cost = positive_ask.price() + negative_ask.price();
    let payout = ctx.payout();

    // No arbitrage if cost >= payout
    if total_cost >= payout {
        return None;
    }

    let edge = payout - total_cost;

    // Skip if edge too small
    if edge < config.min_edge {
        return None;
    }

    // Volume limited by smaller side
    let volume = positive_ask.size().min(negative_ask.size());
    let expected_profit = edge * volume;

    // Skip if profit too small
    if expected_profit < config.min_profit {
        return None;
    }

    // Build opportunity
    use crate::domain::OpportunityLeg;
    let legs = vec![
        OpportunityLeg::new(positive_outcome.token_id().clone(), positive_ask.price()),
        OpportunityLeg::new(negative_outcome.token_id().clone(), negative_ask.price()),
    ];

    Some(Opportunity::with_strategy(
        ctx.market_id().clone(),
        ctx.question(),
        legs,
        volume,
        payout,
        "single_condition",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::strategy::ConcreteDetectionContext;
    use crate::domain::{Book, Market, MarketId, Outcome, PriceLevel, TokenId};
    use crate::infrastructure::cache::BookCache;
    use rust_decimal_macros::dec;

    fn make_market() -> Market {
        let outcomes = vec![
            Outcome::new(TokenId::from("yes-token"), "Yes"),
            Outcome::new(TokenId::from("no-token"), "No"),
        ];
        Market::new(
            MarketId::from("test-market"),
            "Test question?",
            outcomes,
            dec!(1),
        )
    }

    fn make_config() -> SingleConditionConfig {
        SingleConditionConfig {
            min_edge: dec!(0.05),
            min_profit: dec!(0.50),
        }
    }

    #[test]
    fn test_strategy_name() {
        let strategy = SingleConditionStrategy::new(make_config());
        assert_eq!(strategy.name(), "single_condition");
    }

    #[test]
    fn test_applies_to_binary_only() {
        let strategy = SingleConditionStrategy::new(make_config());

        assert!(strategy.applies_to(&MarketContext::binary()));
        assert!(!strategy.applies_to(&MarketContext::multi_outcome(3)));
        assert!(!strategy.applies_to(&MarketContext::multi_outcome(5)));
    }

    #[test]
    fn test_detects_arbitrage_when_sum_below_one() {
        let market = make_market();
        let cache = BookCache::new();
        let config = make_config();

        let outcomes = market.outcomes();
        let positive_token = outcomes[0].token_id();
        let negative_token = outcomes[1].token_id();

        // Positive: 0.40, Negative: 0.50 = 0.90 total (10% edge)
        cache.update(Book::with_levels(
            positive_token.clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.40), dec!(100))],
        ));
        cache.update(Book::with_levels(
            negative_token.clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(100))],
        ));

        let ctx = ConcreteDetectionContext::new(&market, &cache);
        let opp = detect_single_condition(&ctx, &config);
        assert!(opp.is_some());

        let opp = opp.unwrap();
        assert_eq!(opp.edge(), dec!(0.10));
        assert_eq!(opp.total_cost(), dec!(0.90));
        assert_eq!(opp.expected_profit(), dec!(10.00));
    }

    #[test]
    fn test_no_arbitrage_when_sum_equals_one() {
        let market = make_market();
        let cache = BookCache::new();
        let config = make_config();

        let outcomes = market.outcomes();
        let positive_token = outcomes[0].token_id();
        let negative_token = outcomes[1].token_id();

        cache.update(Book::with_levels(
            positive_token.clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(100))],
        ));
        cache.update(Book::with_levels(
            negative_token.clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(100))],
        ));

        let ctx = ConcreteDetectionContext::new(&market, &cache);
        assert!(detect_single_condition(&ctx, &config).is_none());
    }

    #[test]
    fn test_no_arbitrage_when_edge_too_small() {
        let market = make_market();
        let cache = BookCache::new();
        let config = make_config();

        let outcomes = market.outcomes();
        let positive_token = outcomes[0].token_id();
        let negative_token = outcomes[1].token_id();

        // 0.48 + 0.50 = 0.98 (only 2% edge, below 5% threshold)
        cache.update(Book::with_levels(
            positive_token.clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.48), dec!(100))],
        ));
        cache.update(Book::with_levels(
            negative_token.clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(100))],
        ));

        let ctx = ConcreteDetectionContext::new(&market, &cache);
        assert!(detect_single_condition(&ctx, &config).is_none());
    }

    #[test]
    fn test_no_arbitrage_when_profit_too_small() {
        let market = make_market();
        let cache = BookCache::new();
        let config = make_config();

        let outcomes = market.outcomes();
        let positive_token = outcomes[0].token_id();
        let negative_token = outcomes[1].token_id();

        // 10% edge but only 1 share = $0.10 profit (below $0.50 threshold)
        cache.update(Book::with_levels(
            positive_token.clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.40), dec!(1))],
        ));
        cache.update(Book::with_levels(
            negative_token.clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(1))],
        ));

        let ctx = ConcreteDetectionContext::new(&market, &cache);
        assert!(detect_single_condition(&ctx, &config).is_none());
    }

    #[test]
    fn test_volume_limited_by_smaller_side() {
        let market = make_market();
        let cache = BookCache::new();
        let config = make_config();

        let outcomes = market.outcomes();
        let positive_token = outcomes[0].token_id();
        let negative_token = outcomes[1].token_id();

        // Positive has 50, Negative has 100 -> volume = 50
        cache.update(Book::with_levels(
            positive_token.clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.40), dec!(50))],
        ));
        cache.update(Book::with_levels(
            negative_token.clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(100))],
        ));

        let ctx = ConcreteDetectionContext::new(&market, &cache);
        let opp = detect_single_condition(&ctx, &config).unwrap();
        assert_eq!(opp.volume(), dec!(50));
        assert_eq!(opp.expected_profit(), dec!(5.00)); // 50 * 0.10
    }

    #[test]
    fn test_strategy_detect_uses_context() {
        let strategy = SingleConditionStrategy::new(make_config());
        let market = make_market();
        let cache = BookCache::new();

        let outcomes = market.outcomes();
        let positive_token = outcomes[0].token_id();
        let negative_token = outcomes[1].token_id();

        cache.update(Book::with_levels(
            positive_token.clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.40), dec!(100))],
        ));
        cache.update(Book::with_levels(
            negative_token.clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(100))],
        ));

        let ctx = ConcreteDetectionContext::new(&market, &cache);
        let opportunities = strategy.detect(&ctx);

        assert_eq!(opportunities.len(), 1);
    }

    #[test]
    fn test_custom_payout_affects_edge_calculation() {
        // With payout of $100, cost of $90 gives $10 edge (10%)
        let strategy = SingleConditionStrategy::new(SingleConditionConfig {
            min_edge: dec!(5.00), // $5 minimum edge
            min_profit: dec!(0.50),
        });

        let market_outcomes = vec![
            Outcome::new(TokenId::from("yes-token"), "Yes"),
            Outcome::new(TokenId::from("no-token"), "No"),
        ];
        let market_1 = Market::new(
            MarketId::from("test-market"),
            "Test question?",
            market_outcomes.clone(),
            dec!(1),
        );

        let cache = BookCache::new();

        let outcomes = market_1.outcomes();
        let positive_token = outcomes[0].token_id();
        let negative_token = outcomes[1].token_id();

        // Positive: $40, Negative: $50 = $90 total cost
        cache.update(Book::with_levels(
            positive_token.clone(),
            vec![],
            vec![PriceLevel::new(dec!(40), dec!(100))],
        ));
        cache.update(Book::with_levels(
            negative_token.clone(),
            vec![],
            vec![PriceLevel::new(dec!(50), dec!(100))],
        ));

        // With default $1 payout, no opportunity (cost $90 > payout $1)
        let ctx_default = ConcreteDetectionContext::new(&market_1, &cache);
        let opps_default = strategy.detect(&ctx_default);
        assert!(
            opps_default.is_empty(),
            "Should have no opportunity with $1 payout"
        );

        // Create market with $100 payout
        let market_100 = Market::new(
            MarketId::from("test-market"),
            "Test question?",
            market_outcomes,
            dec!(100),
        );

        // With $100 payout, opportunity exists (cost $90 < payout $100)
        let ctx_custom = ConcreteDetectionContext::new(&market_100, &cache);
        let opps_custom = strategy.detect(&ctx_custom);
        assert_eq!(
            opps_custom.len(),
            1,
            "Should have opportunity with $100 payout"
        );

        let opp = &opps_custom[0];
        assert_eq!(opp.edge(), dec!(10));
        assert_eq!(opp.payout(), dec!(100));
    }
}
