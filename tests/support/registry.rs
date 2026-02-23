use edgelord::domain::{market::Market, market::MarketRegistry};

pub fn make_registry(markets: Vec<Market>) -> MarketRegistry {
    let mut registry = MarketRegistry::new();
    for market in markets {
        registry.add(market);
    }
    registry
}
