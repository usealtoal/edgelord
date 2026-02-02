//! Integration tests for strategy system.

use edgelord::domain::strategy::{
    DetectionContext, MarketContext, MarketRebalancingStrategy, SingleConditionConfig,
    SingleConditionStrategy, Strategy, StrategyRegistry,
};
use edgelord::domain::{MarketId, MarketPair, OrderBook, OrderBookCache, PriceLevel, TokenId};
use rust_decimal_macros::dec;

fn make_pair() -> MarketPair {
    MarketPair::new(
        MarketId::from("test-market"),
        "Will it happen?",
        TokenId::from("yes-token"),
        TokenId::from("no-token"),
    )
}

fn setup_arbitrage_books(cache: &OrderBookCache, pair: &MarketPair) {
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
}

fn setup_no_arbitrage_books(cache: &OrderBookCache, pair: &MarketPair) {
    // YES: 0.50, NO: 0.50 = 1.00 total (no edge)
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
}

#[test]
fn test_strategy_registry_detects_with_single_condition() {
    let mut registry = StrategyRegistry::new();
    registry.register(Box::new(SingleConditionStrategy::new(
        SingleConditionConfig::default(),
    )));

    let pair = make_pair();
    let cache = OrderBookCache::new();
    setup_arbitrage_books(&cache, &pair);

    let ctx = DetectionContext::new(&pair, &cache);
    let opportunities = registry.detect_all(&ctx);

    assert_eq!(opportunities.len(), 1);
    assert_eq!(opportunities[0].edge(), dec!(0.10));
}

#[test]
fn test_strategy_registry_empty_when_no_arbitrage() {
    let mut registry = StrategyRegistry::new();
    registry.register(Box::new(SingleConditionStrategy::new(
        SingleConditionConfig::default(),
    )));

    let pair = make_pair();
    let cache = OrderBookCache::new();
    setup_no_arbitrage_books(&cache, &pair);

    let ctx = DetectionContext::new(&pair, &cache);
    let opportunities = registry.detect_all(&ctx);

    assert!(opportunities.is_empty());
}

#[test]
fn test_multiple_strategies_in_registry() {
    let mut registry = StrategyRegistry::new();

    registry.register(Box::new(SingleConditionStrategy::new(
        SingleConditionConfig::default(),
    )));
    // MarketRebalancing won't trigger on binary markets
    registry.register(Box::new(MarketRebalancingStrategy::new(Default::default())));

    assert_eq!(registry.strategies().len(), 2);

    let pair = make_pair();
    let cache = OrderBookCache::new();
    setup_arbitrage_books(&cache, &pair);

    let ctx = DetectionContext::new(&pair, &cache);
    let opportunities = registry.detect_all(&ctx);

    // Only single_condition should fire (binary market)
    assert_eq!(opportunities.len(), 1);
}

#[test]
fn test_strategy_applies_to_filtering() {
    let single = SingleConditionStrategy::new(SingleConditionConfig::default());

    assert!(single.applies_to(&MarketContext::binary()));
    assert!(!single.applies_to(&MarketContext::multi_outcome(3)));
}

#[test]
fn test_market_rebalancing_applies_to_multi_outcome() {
    let rebalancing = MarketRebalancingStrategy::new(Default::default());

    assert!(!rebalancing.applies_to(&MarketContext::binary()));
    assert!(rebalancing.applies_to(&MarketContext::multi_outcome(3)));
    assert!(rebalancing.applies_to(&MarketContext::multi_outcome(5)));
}

#[test]
fn test_empty_registry_returns_no_opportunities() {
    let registry = StrategyRegistry::new();

    let pair = make_pair();
    let cache = OrderBookCache::new();
    setup_arbitrage_books(&cache, &pair);

    let ctx = DetectionContext::new(&pair, &cache);
    let opportunities = registry.detect_all(&ctx);

    assert!(opportunities.is_empty());
}
