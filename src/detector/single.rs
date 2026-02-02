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
