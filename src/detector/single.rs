use rust_decimal::Decimal;

use crate::orderbook::{MarketRegistry, OrderBookCache};
use crate::types::{MarketPair, Opportunity};

use super::DetectorConfig;

/// Detect single-condition arbitrage (YES + NO < $1.00)
pub fn detect_single_condition(
    pair: &MarketPair,
    cache: &OrderBookCache,
    config: &DetectorConfig,
) -> Option<Opportunity> {
    // Get both order books atomically
    let (yes_book, no_book) = cache.get_pair(&pair.yes_token, &pair.no_token);

    let yes_book = yes_book?;
    let no_book = no_book?;

    // Get best asks (what we'd pay to buy)
    let yes_ask = yes_book.best_ask()?;
    let no_ask = no_book.best_ask()?;

    // Calculate edge
    let total_cost = yes_ask.price + no_ask.price;
    let one = Decimal::ONE;

    // If total cost >= $1, no arbitrage
    if total_cost >= one {
        return None;
    }

    let edge = one - total_cost;

    // Check minimum edge
    if edge < config.min_edge {
        return None;
    }

    // Volume is limited by smaller side
    let volume = yes_ask.size.min(no_ask.size);

    // Expected profit
    let expected_profit = edge * volume;

    // Check minimum profit
    if expected_profit < config.min_profit {
        return None;
    }

    Some(Opportunity {
        market_id: pair.market_id.clone(),
        question: pair.question.clone(),
        yes_token: pair.yes_token.clone(),
        no_token: pair.no_token.clone(),
        yes_ask: yes_ask.price,
        no_ask: no_ask.price,
        total_cost,
        edge,
        volume,
        expected_profit,
    })
}

/// Scan all markets for arbitrage opportunities
#[allow(dead_code)]
pub fn scan_all(
    registry: &MarketRegistry,
    cache: &OrderBookCache,
    config: &DetectorConfig,
) -> Vec<Opportunity> {
    registry
        .pairs()
        .iter()
        .filter_map(|pair| detect_single_condition(pair, cache, config))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MarketId, OrderBook, PriceLevel, TokenId};
    use rust_decimal_macros::dec;

    fn make_pair() -> MarketPair {
        MarketPair {
            market_id: MarketId::from("test-market".to_string()),
            question: "Test question?".to_string(),
            yes_token: TokenId::from("yes-token"),
            no_token: TokenId::from("no-token"),
        }
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

        // YES ask: 0.40, NO ask: 0.50 -> total 0.90, edge 0.10
        let yes_book = OrderBook {
            token_id: pair.yes_token.clone(),
            bids: vec![],
            asks: vec![PriceLevel {
                price: dec!(0.40),
                size: dec!(100),
            }],
        };
        let no_book = OrderBook {
            token_id: pair.no_token.clone(),
            bids: vec![],
            asks: vec![PriceLevel {
                price: dec!(0.50),
                size: dec!(100),
            }],
        };

        cache.books.write().insert(pair.yes_token.clone(), yes_book);
        cache.books.write().insert(pair.no_token.clone(), no_book);

        let opp = detect_single_condition(&pair, &cache, &config);
        assert!(opp.is_some());

        let opp = opp.unwrap();
        assert_eq!(opp.edge, dec!(0.10));
        assert_eq!(opp.total_cost, dec!(0.90));
        assert_eq!(opp.expected_profit, dec!(10.00)); // 0.10 * 100
    }

    #[test]
    fn test_no_arbitrage_when_sum_equals_one() {
        let pair = make_pair();
        let cache = OrderBookCache::new();
        let config = make_config();

        // YES ask: 0.50, NO ask: 0.50 -> total 1.00, no edge
        let yes_book = OrderBook {
            token_id: pair.yes_token.clone(),
            bids: vec![],
            asks: vec![PriceLevel {
                price: dec!(0.50),
                size: dec!(100),
            }],
        };
        let no_book = OrderBook {
            token_id: pair.no_token.clone(),
            bids: vec![],
            asks: vec![PriceLevel {
                price: dec!(0.50),
                size: dec!(100),
            }],
        };

        cache.books.write().insert(pair.yes_token.clone(), yes_book);
        cache.books.write().insert(pair.no_token.clone(), no_book);

        let opp = detect_single_condition(&pair, &cache, &config);
        assert!(opp.is_none());
    }

    #[test]
    fn test_no_arbitrage_when_edge_too_small() {
        let pair = make_pair();
        let cache = OrderBookCache::new();
        let config = make_config(); // min_edge = 0.05

        // YES ask: 0.48, NO ask: 0.50 -> total 0.98, edge 0.02 (below min)
        let yes_book = OrderBook {
            token_id: pair.yes_token.clone(),
            bids: vec![],
            asks: vec![PriceLevel {
                price: dec!(0.48),
                size: dec!(100),
            }],
        };
        let no_book = OrderBook {
            token_id: pair.no_token.clone(),
            bids: vec![],
            asks: vec![PriceLevel {
                price: dec!(0.50),
                size: dec!(100),
            }],
        };

        cache.books.write().insert(pair.yes_token.clone(), yes_book);
        cache.books.write().insert(pair.no_token.clone(), no_book);

        let opp = detect_single_condition(&pair, &cache, &config);
        assert!(opp.is_none());
    }

    #[test]
    fn test_no_arbitrage_when_profit_too_small() {
        let pair = make_pair();
        let cache = OrderBookCache::new();
        let config = make_config(); // min_profit = 0.50

        // YES ask: 0.40, NO ask: 0.50 -> edge 0.10, but volume only 1
        // expected profit = 0.10 * 1 = 0.10 (below min)
        let yes_book = OrderBook {
            token_id: pair.yes_token.clone(),
            bids: vec![],
            asks: vec![PriceLevel {
                price: dec!(0.40),
                size: dec!(1),
            }],
        };
        let no_book = OrderBook {
            token_id: pair.no_token.clone(),
            bids: vec![],
            asks: vec![PriceLevel {
                price: dec!(0.50),
                size: dec!(1),
            }],
        };

        cache.books.write().insert(pair.yes_token.clone(), yes_book);
        cache.books.write().insert(pair.no_token.clone(), no_book);

        let opp = detect_single_condition(&pair, &cache, &config);
        assert!(opp.is_none());
    }

    #[test]
    fn test_volume_limited_by_smaller_side() {
        let pair = make_pair();
        let cache = OrderBookCache::new();
        let config = make_config();

        // YES has 50 volume, NO has 100 volume -> should use 50
        let yes_book = OrderBook {
            token_id: pair.yes_token.clone(),
            bids: vec![],
            asks: vec![PriceLevel {
                price: dec!(0.40),
                size: dec!(50),
            }],
        };
        let no_book = OrderBook {
            token_id: pair.no_token.clone(),
            bids: vec![],
            asks: vec![PriceLevel {
                price: dec!(0.50),
                size: dec!(100),
            }],
        };

        cache.books.write().insert(pair.yes_token.clone(), yes_book);
        cache.books.write().insert(pair.no_token.clone(), no_book);

        let opp = detect_single_condition(&pair, &cache, &config);
        assert!(opp.is_some());

        let opp = opp.unwrap();
        assert_eq!(opp.volume, dec!(50));
        assert_eq!(opp.expected_profit, dec!(5.00)); // 0.10 * 50
    }
}
