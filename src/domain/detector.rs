//! Arbitrage detection logic.

use rust_decimal::Decimal;
use serde::Deserialize;

use super::{MarketPair, Opportunity, OrderBookCache};

/// Configuration for the arbitrage detector
#[derive(Debug, Clone, Deserialize)]
pub struct DetectorConfig {
    /// Minimum edge (profit per $1) to consider an opportunity
    #[serde(default = "default_min_edge")]
    pub min_edge: Decimal,

    /// Minimum expected profit in dollars to act on
    #[serde(default = "default_min_profit")]
    pub min_profit: Decimal,
}

fn default_min_edge() -> Decimal {
    Decimal::new(5, 2) // 0.05
}

fn default_min_profit() -> Decimal {
    Decimal::new(50, 2) // 0.50
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            min_edge: default_min_edge(),
            min_profit: default_min_profit(),
        }
    }
}

/// Detect single-condition arbitrage (YES + NO < $1.00)
pub fn detect_single_condition(
    pair: &MarketPair,
    cache: &OrderBookCache,
    config: &DetectorConfig,
) -> Option<Opportunity> {
    let (yes_book, no_book) = cache.get_pair(pair.yes_token(), pair.no_token());

    let yes_book = yes_book?;
    let no_book = no_book?;

    let yes_ask = yes_book.best_ask()?;
    let no_ask = no_book.best_ask()?;

    let total_cost = yes_ask.price() + no_ask.price();
    let one = Decimal::ONE;

    if total_cost >= one {
        return None;
    }

    let edge = one - total_cost;

    if edge < config.min_edge {
        return None;
    }

    let volume = yes_ask.size().min(no_ask.size());
    let expected_profit = edge * volume;

    if expected_profit < config.min_profit {
        return None;
    }

    // Build the opportunity using the builder pattern
    // The builder calculates total_cost, edge, and expected_profit internally
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

    fn make_config() -> DetectorConfig {
        DetectorConfig {
            min_edge: dec!(0.05),
            min_profit: dec!(0.50),
        }
    }

    #[test]
    fn test_detects_arbitrage_when_sum_below_one() {
        let pair = make_pair();
        let cache = OrderBookCache::new();
        let config = make_config();

        let yes_book = OrderBook::with_levels(
            pair.yes_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.40), dec!(100))],
        );
        let no_book = OrderBook::with_levels(
            pair.no_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(100))],
        );

        cache.update(yes_book);
        cache.update(no_book);

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

        let yes_book = OrderBook::with_levels(
            pair.yes_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(100))],
        );
        let no_book = OrderBook::with_levels(
            pair.no_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(100))],
        );

        cache.update(yes_book);
        cache.update(no_book);

        let opp = detect_single_condition(&pair, &cache, &config);
        assert!(opp.is_none());
    }

    #[test]
    fn test_no_arbitrage_when_edge_too_small() {
        let pair = make_pair();
        let cache = OrderBookCache::new();
        let config = make_config();

        let yes_book = OrderBook::with_levels(
            pair.yes_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.48), dec!(100))],
        );
        let no_book = OrderBook::with_levels(
            pair.no_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(100))],
        );

        cache.update(yes_book);
        cache.update(no_book);

        let opp = detect_single_condition(&pair, &cache, &config);
        assert!(opp.is_none());
    }

    #[test]
    fn test_no_arbitrage_when_profit_too_small() {
        let pair = make_pair();
        let cache = OrderBookCache::new();
        let config = make_config();

        let yes_book = OrderBook::with_levels(
            pair.yes_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.40), dec!(1))],
        );
        let no_book = OrderBook::with_levels(
            pair.no_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(1))],
        );

        cache.update(yes_book);
        cache.update(no_book);

        let opp = detect_single_condition(&pair, &cache, &config);
        assert!(opp.is_none());
    }

    #[test]
    fn test_volume_limited_by_smaller_side() {
        let pair = make_pair();
        let cache = OrderBookCache::new();
        let config = make_config();

        let yes_book = OrderBook::with_levels(
            pair.yes_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.40), dec!(50))],
        );
        let no_book = OrderBook::with_levels(
            pair.no_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(100))],
        );

        cache.update(yes_book);
        cache.update(no_book);

        let opp = detect_single_condition(&pair, &cache, &config);
        assert!(opp.is_some());

        let opp = opp.unwrap();
        assert_eq!(opp.volume(), dec!(50));
        assert_eq!(opp.expected_profit(), dec!(5.00));
    }
}
