//! Integration tests for multi-exchange abstraction.

use edgelord::core::domain::{
    Market, MarketId, MarketRegistry, Opportunity, OpportunityLeg, Outcome, TokenId,
};
use edgelord::core::exchange::polymarket::PolymarketExchangeConfig;
use edgelord::core::exchange::{ExchangeConfig, MarketInfo, OutcomeInfo};
use rust_decimal_macros::dec;

#[test]
fn test_polymarket_config_values() {
    let config = PolymarketExchangeConfig;

    assert_eq!(config.name(), "polymarket");
    assert_eq!(config.default_payout(), dec!(1.00));
    assert_eq!(config.binary_outcome_names(), ("Yes", "No"));
}

#[test]
fn test_exchange_config_parses_binary_markets() {
    let config = PolymarketExchangeConfig;

    let market_infos = vec![MarketInfo {
        id: "market-1".to_string(),
        question: "Will it rain?".to_string(),
        outcomes: vec![
            OutcomeInfo {
                token_id: "yes-token".to_string(),
                name: "Yes".to_string(),
                price: None,
            },
            OutcomeInfo {
                token_id: "no-token".to_string(),
                name: "No".to_string(),
                price: None,
            },
        ],
        active: true,
    }];

    let markets = config.parse_markets(&market_infos);

    assert_eq!(markets.len(), 1);
    assert!(markets[0].is_binary());
    assert_eq!(markets[0].payout(), dec!(1.00));
}

#[test]
fn test_generic_market_registry_workflow() {
    let mut registry = MarketRegistry::new();

    // Add binary market
    let binary = Market::new(
        MarketId::from("m1"),
        "Will it rain?",
        vec![
            Outcome::new(TokenId::from("yes-1"), "Yes"),
            Outcome::new(TokenId::from("no-1"), "No"),
        ],
        dec!(1.00),
    );
    registry.add(binary);

    // Add multi-outcome market
    let multi = Market::new(
        MarketId::from("m2"),
        "Who wins?",
        vec![
            Outcome::new(TokenId::from("trump"), "Trump"),
            Outcome::new(TokenId::from("biden"), "Biden"),
            Outcome::new(TokenId::from("other"), "Other"),
        ],
        dec!(1.00),
    );
    registry.add(multi);

    // Verify lookups work
    assert_eq!(registry.len(), 2);
    assert!(registry.get_by_token(&TokenId::from("yes-1")).is_some());
    assert!(registry.get_by_token(&TokenId::from("trump")).is_some());
    assert!(registry.get_by_token(&TokenId::from("unknown")).is_none());

    // Verify filtering works
    assert_eq!(registry.binary_markets().count(), 1);
    assert_eq!(registry.multi_outcome_markets().count(), 1);
}

#[test]
fn test_opportunity_with_configurable_payout() {
    // Standard $1 payout
    let opp1 = Opportunity::new(
        MarketId::from("m1"),
        "Q?",
        vec![
            OpportunityLeg::new(TokenId::from("yes"), dec!(0.45)),
            OpportunityLeg::new(TokenId::from("no"), dec!(0.45)),
        ],
        dec!(100),
        dec!(1.00),
    );
    assert_eq!(opp1.total_cost(), dec!(0.90));
    assert_eq!(opp1.edge(), dec!(0.10));
    assert_eq!(opp1.expected_profit(), dec!(10.00));

    // Custom $10 payout (hypothetical exchange)
    let opp2 = Opportunity::new(
        MarketId::from("m2"),
        "Q?",
        vec![
            OpportunityLeg::new(TokenId::from("a"), dec!(4.50)),
            OpportunityLeg::new(TokenId::from("b"), dec!(4.50)),
        ],
        dec!(10),
        dec!(10.00),
    );
    assert_eq!(opp2.total_cost(), dec!(9.00));
    assert_eq!(opp2.edge(), dec!(1.00));
    assert_eq!(opp2.expected_profit(), dec!(10.00));
}

#[test]
fn test_market_payout_flows_through() {
    // Create market with specific payout
    let market = Market::new(
        MarketId::from("m1"),
        "Test?",
        vec![
            Outcome::new(TokenId::from("yes"), "Yes"),
            Outcome::new(TokenId::from("no"), "No"),
        ],
        dec!(5.00), // Custom payout
    );

    // Payout is accessible
    assert_eq!(market.payout(), dec!(5.00));

    // Registry preserves payout
    let mut registry = MarketRegistry::new();
    registry.add(market);

    let retrieved = registry.get_by_token(&TokenId::from("yes")).unwrap();
    assert_eq!(retrieved.payout(), dec!(5.00));
}
