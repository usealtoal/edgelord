//! Integration tests for strategy system.

mod support;

use edgelord::adapter::strategy::{
    ConcreteDetectionContext, MarketRebalancingStrategy, SingleConditionConfig,
    SingleConditionStrategy, StrategyRegistry,
};
use edgelord::port::{DetectionContext, MarketContext, Strategy};
use edgelord::domain::{Market, Book, PriceLevel};
use edgelord::runtime::cache::BookCache;
use rust_decimal_macros::dec;

fn setup_arbitrage_books(cache: &BookCache, market: &Market) {
    let yes_token = market.outcome_by_name("Yes").unwrap().token_id();
    let no_token = market.outcome_by_name("No").unwrap().token_id();

    // YES: 0.40, NO: 0.50 = 0.90 total (10% edge)
    cache.update(Book::with_levels(
        yes_token.clone(),
        vec![],
        vec![PriceLevel::new(dec!(0.40), dec!(100))],
    ));
    cache.update(Book::with_levels(
        no_token.clone(),
        vec![],
        vec![PriceLevel::new(dec!(0.50), dec!(100))],
    ));
}

fn setup_no_arbitrage_books(cache: &BookCache, market: &Market) {
    let yes_token = market.outcome_by_name("Yes").unwrap().token_id();
    let no_token = market.outcome_by_name("No").unwrap().token_id();

    // YES: 0.50, NO: 0.50 = 1.00 total (no edge)
    cache.update(Book::with_levels(
        yes_token.clone(),
        vec![],
        vec![PriceLevel::new(dec!(0.50), dec!(100))],
    ));
    cache.update(Book::with_levels(
        no_token.clone(),
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

    let market = support::market::make_binary_market(
        "test-market",
        "Will it happen?",
        "yes-token",
        "no-token",
        dec!(1),
    );
    let cache = BookCache::new();
    setup_arbitrage_books(&cache, &market);

    let ctx = ConcreteDetectionContext::new(&market, &cache);
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

    let market = support::market::make_binary_market(
        "test-market",
        "Will it happen?",
        "yes-token",
        "no-token",
        dec!(1),
    );
    let cache = BookCache::new();
    setup_no_arbitrage_books(&cache, &market);

    let ctx = ConcreteDetectionContext::new(&market, &cache);
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

    let market = support::market::make_binary_market(
        "test-market",
        "Will it happen?",
        "yes-token",
        "no-token",
        dec!(1),
    );
    let cache = BookCache::new();
    setup_arbitrage_books(&cache, &market);

    let ctx = ConcreteDetectionContext::new(&market, &cache);
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

    let market = support::market::make_binary_market(
        "test-market",
        "Will it happen?",
        "yes-token",
        "no-token",
        dec!(1),
    );
    let cache = BookCache::new();
    setup_arbitrage_books(&cache, &market);

    let ctx = ConcreteDetectionContext::new(&market, &cache);
    let opportunities = registry.detect_all(&ctx);

    assert!(opportunities.is_empty());
}

#[test]
fn strategy_skips_when_order_books_missing() {
    // Test single-condition strategy with missing order books
    let mut registry = StrategyRegistry::new();
    registry.register(Box::new(SingleConditionStrategy::new(
        SingleConditionConfig::default(),
    )));

    let market = support::market::make_binary_market(
        "test-market",
        "Will it happen?",
        "yes-token",
        "no-token",
        dec!(1),
    );
    let cache = BookCache::new();
    // Don't add any order books - cache is empty

    let ctx = ConcreteDetectionContext::new(&market, &cache);
    let opportunities = registry.detect_all(&ctx);

    assert!(
        opportunities.is_empty(),
        "Single-condition strategy should return no opportunities when order books are missing"
    );

    // Test market rebalancing strategy with missing order books
    let mut registry_rebal = StrategyRegistry::new();
    registry_rebal.register(Box::new(MarketRebalancingStrategy::new(Default::default())));

    let multi_market = support::market::make_multi_market(
        "multi-market",
        "Who wins?",
        &[
            ("token-a", "Option A"),
            ("token-b", "Option B"),
            ("token-c", "Option C"),
        ],
        dec!(1),
    );
    let cache_rebal = BookCache::new();
    // Don't add any order books - cache is empty

    let ctx_rebal = ConcreteDetectionContext::new(&multi_market, &cache_rebal);
    let opportunities_rebal = registry_rebal.detect_all(&ctx_rebal);

    assert!(
        opportunities_rebal.is_empty(),
        "Market rebalancing strategy should return no opportunities when order books are missing"
    );
}
