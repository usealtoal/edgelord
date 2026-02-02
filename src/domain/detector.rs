//! Arbitrage detection (legacy module).
//!
//! This module is deprecated. Use `domain::strategy` instead.
//!
//! Re-exports are provided for backwards compatibility.

pub use crate::domain::strategy::single_condition::{
    detect_single_condition, SingleConditionConfig as DetectorConfig,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{MarketId, MarketPair, OrderBook, OrderBookCache, PriceLevel, TokenId};
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
