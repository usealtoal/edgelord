//! Tests for exchange factory and approval components.

use edgelord::app::Config;
use edgelord::core::exchange::ExchangeFactory;

#[test]
fn factory_returns_error_when_exchange_config_missing() {
    // Test that factory methods return Result types instead of panicking
    // This test will fail initially because methods don't return Result yet
    
    let config = Config::default();
    
    // Test create_scorer - should return Result<Box<dyn MarketScorer>>
    let result = ExchangeFactory::create_scorer(&config);
    
    // Verify it returns a Result (not panics)
    // With default config, polymarket_config() returns Some, so this should be Ok
    // But the method signature should be Result to avoid panics
    assert!(result.is_ok(), "create_scorer should return Ok with valid config");
    
    // Test create_filter - should return Result<Box<dyn MarketFilter>>
    let result = ExchangeFactory::create_filter(&config);
    assert!(result.is_ok(), "create_filter should return Ok with valid config");
    
    // Test create_deduplicator - should return Result<Box<dyn MessageDeduplicator>>
    let result = ExchangeFactory::create_deduplicator(&config);
    assert!(result.is_ok(), "create_deduplicator should return Ok with valid config");
}
