//! Arbitrage detection logic for prediction markets.
//!
//! This module implements single-condition arbitrage detection, which identifies
//! opportunities where the sum of YES and NO ask prices is less than $1.00.
//! In a well-functioning binary prediction market, YES + NO should equal $1.00,
//! so any discount represents a risk-free profit opportunity.
//!
//! # Detection Strategy
//!
//! The detector scans market pairs and identifies opportunities based on:
//! - **Edge**: The profit margin per dollar (1.0 - total_cost)
//! - **Volume**: Minimum of available YES and NO liquidity
//! - **Expected Profit**: Edge multiplied by tradeable volume
//!
//! Both minimum edge and minimum expected profit thresholds can be configured
//! to filter out opportunities that are too small to be worthwhile.
//!
//! # Example
//!
//! ```ignore
//! use edgelord::domain::{DetectorConfig, MarketPair, OrderBookCache};
//! use edgelord::domain::detector::detect_single_condition;
//!
//! let pair = MarketPair::new(market_id, "Will it rain?", yes_token, no_token);
//! let cache = OrderBookCache::new();
//! // ... populate cache with order books ...
//! let config = DetectorConfig::default();
//!
//! if let Some(opportunity) = detect_single_condition(&pair, &cache, &config) {
//!     println!("Found opportunity with edge: {}", opportunity.edge());
//! }
//! ```

use rust_decimal::Decimal;
use serde::Deserialize;

use super::{MarketPair, Opportunity, OrderBookCache};

/// Configuration for the arbitrage detector.
///
/// Controls the minimum thresholds for considering an opportunity actionable.
/// Default values are conservative to avoid chasing tiny profits.
///
/// # Example
///
/// ```
/// use edgelord::domain::DetectorConfig;
/// use rust_decimal_macros::dec;
///
/// // Use defaults
/// let config = DetectorConfig::default();
///
/// // Or customize thresholds
/// let config = DetectorConfig {
///     min_edge: dec!(0.03),      // 3% minimum edge
///     min_profit: dec!(1.00),    // $1.00 minimum expected profit
/// };
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct DetectorConfig {
    /// Minimum edge (profit per $1) to consider an opportunity.
    ///
    /// Default: 0.05 (5% edge required)
    #[serde(default = "default_min_edge")]
    pub min_edge: Decimal,

    /// Minimum expected profit in dollars to act on.
    ///
    /// Default: 0.50 ($0.50 minimum profit)
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

/// Detect single-condition arbitrage opportunities.
///
/// Checks if the sum of the best YES ask and best NO ask prices is less than $1.00.
/// If so, buying both outcomes guarantees a profit since they resolve to exactly $1.00.
///
/// # Arguments
///
/// * `pair` - The market pair containing YES and NO token IDs
/// * `cache` - Order book cache with current market data
/// * `config` - Detection thresholds (min edge, min profit)
///
/// # Returns
///
/// Returns `Some(Opportunity)` if an actionable arbitrage exists, `None` otherwise.
///
/// # Example
///
/// ```ignore
/// let opportunity = detect_single_condition(&pair, &cache, &config);
/// if let Some(opp) = opportunity {
///     println!("Market: {}", opp.question());
///     println!("Edge: {}%", opp.edge() * 100);
///     println!("Expected profit: ${}", opp.expected_profit());
/// }
/// ```
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
