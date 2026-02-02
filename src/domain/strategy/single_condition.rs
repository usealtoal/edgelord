//! Single-condition arbitrage strategy.
//!
//! Detects when YES + NO < $1.00 for binary markets.
//! This captured 26.7% ($10.5M) of historical arbitrage profits.

use rust_decimal::Decimal;
use serde::Deserialize;

use super::{DetectionContext, MarketContext, Strategy};
use crate::domain::{MarketPair, Opportunity, OrderBookCache};

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
    Decimal::new(5, 2) // 0.05
}

fn default_min_profit() -> Decimal {
    Decimal::new(50, 2) // 0.50
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

    fn detect(&self, ctx: &DetectionContext) -> Vec<Opportunity> {
        detect_single_condition(ctx.pair, ctx.cache, &self.config)
            .into_iter()
            .collect()
    }
}

/// Core detection logic for single-condition arbitrage.
///
/// Checks if YES ask + NO ask < $1.00, indicating risk-free profit.
///
/// # Arguments
/// * `pair` - The YES/NO market pair
/// * `cache` - Order book cache with current prices
/// * `config` - Detection thresholds
///
/// # Returns
/// `Some(Opportunity)` if arbitrage exists, `None` otherwise.
pub fn detect_single_condition(
    pair: &MarketPair,
    cache: &OrderBookCache,
    config: &SingleConditionConfig,
) -> Option<Opportunity> {
    let (yes_book, no_book) = cache.get_pair(pair.yes_token(), pair.no_token());

    let yes_book = yes_book?;
    let no_book = no_book?;

    let yes_ask = yes_book.best_ask()?;
    let no_ask = no_book.best_ask()?;

    let total_cost = yes_ask.price() + no_ask.price();

    // No arbitrage if cost >= $1
    if total_cost >= Decimal::ONE {
        return None;
    }

    let edge = Decimal::ONE - total_cost;

    // Skip if edge too small
    if edge < config.min_edge {
        return None;
    }

    // Volume limited by smaller side
    let volume = yes_ask.size().min(no_ask.size());
    let expected_profit = edge * volume;

    // Skip if profit too small
    if expected_profit < config.min_profit {
        return None;
    }

    // Build opportunity using the builder pattern
    Opportunity::builder()
        .market_id(pair.market_id().clone())
        .question(pair.question())
        .yes_token(pair.yes_token().clone(), yes_ask.price())
        .no_token(pair.no_token().clone(), no_ask.price())
        .volume(volume)
        .build()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{MarketId, OrderBook, PriceLevel, TokenId};
    use rust_decimal_macros::dec;

    fn make_pair() -> MarketPair {
        MarketPair::new(
            MarketId::from("test-market"),
            "Test question?",
            TokenId::from("yes-token"),
            TokenId::from("no-token"),
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
        let pair = make_pair();
        let cache = OrderBookCache::new();
        let config = make_config();

        // YES: 0.40, NO: 0.50 = 0.90 total (10% edge)
        cache.update(OrderBook::with_levels(
            pair.yes_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.40), dec!(100))],
        ));
        cache.update(OrderBook::with_levels(
            pair.no_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(100))],
        ));

        let opp = detect_single_condition(&pair, &cache, &config);
        assert!(opp.is_some());

        let opp = opp.unwrap();
        assert_eq!(opp.edge(), dec!(0.10));
        assert_eq!(opp.total_cost(), dec!(0.90));
        assert_eq!(opp.expected_profit(), dec!(10.00));
    }

    #[test]
    fn test_no_arbitrage_when_sum_equals_one() {
        let pair = make_pair();
        let cache = OrderBookCache::new();
        let config = make_config();

        cache.update(OrderBook::with_levels(
            pair.yes_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(100))],
        ));
        cache.update(OrderBook::with_levels(
            pair.no_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(100))],
        ));

        assert!(detect_single_condition(&pair, &cache, &config).is_none());
    }

    #[test]
    fn test_no_arbitrage_when_edge_too_small() {
        let pair = make_pair();
        let cache = OrderBookCache::new();
        let config = make_config();

        // 0.48 + 0.50 = 0.98 (only 2% edge, below 5% threshold)
        cache.update(OrderBook::with_levels(
            pair.yes_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.48), dec!(100))],
        ));
        cache.update(OrderBook::with_levels(
            pair.no_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(100))],
        ));

        assert!(detect_single_condition(&pair, &cache, &config).is_none());
    }

    #[test]
    fn test_no_arbitrage_when_profit_too_small() {
        let pair = make_pair();
        let cache = OrderBookCache::new();
        let config = make_config();

        // 10% edge but only 1 share = $0.10 profit (below $0.50 threshold)
        cache.update(OrderBook::with_levels(
            pair.yes_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.40), dec!(1))],
        ));
        cache.update(OrderBook::with_levels(
            pair.no_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(1))],
        ));

        assert!(detect_single_condition(&pair, &cache, &config).is_none());
    }

    #[test]
    fn test_volume_limited_by_smaller_side() {
        let pair = make_pair();
        let cache = OrderBookCache::new();
        let config = make_config();

        // YES has 50, NO has 100 -> volume = 50
        cache.update(OrderBook::with_levels(
            pair.yes_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.40), dec!(50))],
        ));
        cache.update(OrderBook::with_levels(
            pair.no_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(100))],
        ));

        let opp = detect_single_condition(&pair, &cache, &config).unwrap();
        assert_eq!(opp.volume(), dec!(50));
        assert_eq!(opp.expected_profit(), dec!(5.00)); // 50 * 0.10
    }

    #[test]
    fn test_strategy_detect_uses_context() {
        let strategy = SingleConditionStrategy::new(make_config());
        let pair = make_pair();
        let cache = OrderBookCache::new();

        cache.update(OrderBook::with_levels(
            pair.yes_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.40), dec!(100))],
        ));
        cache.update(OrderBook::with_levels(
            pair.no_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(100))],
        ));

        let ctx = DetectionContext::new(&pair, &cache);
        let opportunities = strategy.detect(&ctx);

        assert_eq!(opportunities.len(), 1);
    }
}
