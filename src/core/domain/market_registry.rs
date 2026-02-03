//! Generic market registry supporting any outcome structure.
//!
//! Unlike the Polymarket-specific registry that only handles YES/NO markets,
//! this generic registry works with any number of outcomes per market.

use std::collections::HashMap;

use super::id::TokenId;
use super::market::Market;

/// A registry for markets that supports any outcome structure.
///
/// This registry maps token IDs to their containing markets, enabling
/// efficient lookup from order book events. Works with markets of any size
/// (binary, multi-outcome, etc.).
#[derive(Debug, Default)]
pub struct MarketRegistry {
    token_to_market: HashMap<TokenId, Market>,
    markets: Vec<Market>,
}

impl MarketRegistry {
    /// Create an empty market registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            token_to_market: HashMap::new(),
            markets: Vec::new(),
        }
    }

    /// Add a market to the registry, indexing all its token IDs.
    pub fn add(&mut self, market: Market) {
        // Index all token IDs
        for outcome in market.outcomes() {
            self.token_to_market
                .insert(outcome.token_id().clone(), market.clone());
        }
        self.markets.push(market);
    }

    /// Look up the market for a given token ID.
    ///
    /// Returns the market if the token is registered, or `None`
    /// if the token is not found in any registered market.
    #[must_use]
    pub fn get_by_token(&self, token_id: &TokenId) -> Option<&Market> {
        self.token_to_market.get(token_id)
    }

    /// Get all registered markets.
    #[must_use]
    pub fn markets(&self) -> &[Market] {
        &self.markets
    }

    /// Get markets with exactly 2 outcomes (binary markets).
    pub fn binary_markets(&self) -> impl Iterator<Item = &Market> {
        self.markets.iter().filter(|m| m.outcome_count() == 2)
    }

    /// Get markets with 3 or more outcomes.
    pub fn multi_outcome_markets(&self) -> impl Iterator<Item = &Market> {
        self.markets.iter().filter(|m| m.outcome_count() >= 3)
    }

    /// Get all token IDs across all registered markets.
    pub fn all_token_ids(&self) -> impl Iterator<Item = &TokenId> {
        self.token_to_market.keys()
    }

    /// Get the number of registered markets.
    #[must_use]
    pub fn len(&self) -> usize {
        self.markets.len()
    }

    /// Check if the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.markets.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::domain::{MarketId, Outcome};
    use rust_decimal_macros::dec;

    // --- Helper functions ---

    fn create_binary_market(id: &str, yes_token: &str, no_token: &str) -> Market {
        let outcomes = vec![
            Outcome::new(TokenId::from(yes_token), "Yes"),
            Outcome::new(TokenId::from(no_token), "No"),
        ];
        Market::new(
            MarketId::from(id),
            format!("Binary market {id}?"),
            outcomes,
            dec!(1.00),
        )
    }

    fn create_multi_outcome_market(id: &str, tokens: &[(&str, &str)]) -> Market {
        let outcomes = tokens
            .iter()
            .map(|(token, name)| Outcome::new(TokenId::from(*token), *name))
            .collect();
        Market::new(
            MarketId::from(id),
            format!("Multi-outcome market {id}?"),
            outcomes,
            dec!(1.00),
        )
    }

    // --- Tests ---

    #[test]
    fn new_creates_empty_registry() {
        let registry = MarketRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert_eq!(registry.markets().len(), 0);
    }

    #[test]
    fn default_creates_empty_registry() {
        let registry = MarketRegistry::default();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn add_indexes_all_token_ids() {
        let mut registry = MarketRegistry::new();
        let market = create_binary_market("m1", "yes-1", "no-1");

        registry.add(market);

        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());

        // Both tokens should be indexed
        let yes_token = TokenId::from("yes-1");
        let no_token = TokenId::from("no-1");
        assert!(registry.get_by_token(&yes_token).is_some());
        assert!(registry.get_by_token(&no_token).is_some());
    }

    #[test]
    fn add_multiple_markets() {
        let mut registry = MarketRegistry::new();
        registry.add(create_binary_market("m1", "yes-1", "no-1"));
        registry.add(create_binary_market("m2", "yes-2", "no-2"));

        assert_eq!(registry.len(), 2);
        assert_eq!(registry.markets().len(), 2);
    }

    #[test]
    fn get_by_token_returns_correct_market() {
        let mut registry = MarketRegistry::new();
        registry.add(create_binary_market("m1", "yes-1", "no-1"));
        registry.add(create_binary_market("m2", "yes-2", "no-2"));

        let market = registry.get_by_token(&TokenId::from("yes-1")).unwrap();
        assert_eq!(market.market_id().as_str(), "m1");

        let market = registry.get_by_token(&TokenId::from("no-2")).unwrap();
        assert_eq!(market.market_id().as_str(), "m2");
    }

    #[test]
    fn get_by_token_returns_none_for_unknown() {
        let mut registry = MarketRegistry::new();
        registry.add(create_binary_market("m1", "yes-1", "no-1"));

        assert!(registry.get_by_token(&TokenId::from("unknown")).is_none());
    }

    #[test]
    fn get_by_token_both_tokens_map_to_same_market() {
        let mut registry = MarketRegistry::new();
        registry.add(create_binary_market("m1", "yes-1", "no-1"));

        let yes_market = registry.get_by_token(&TokenId::from("yes-1")).unwrap();
        let no_market = registry.get_by_token(&TokenId::from("no-1")).unwrap();
        assert_eq!(yes_market.market_id(), no_market.market_id());
    }

    #[test]
    fn markets_returns_all_markets() {
        let mut registry = MarketRegistry::new();
        registry.add(create_binary_market("m1", "yes-1", "no-1"));
        registry.add(create_multi_outcome_market(
            "m2",
            &[("red", "Red"), ("blue", "Blue"), ("green", "Green")],
        ));

        let markets = registry.markets();
        assert_eq!(markets.len(), 2);
    }

    #[test]
    fn binary_markets_filters_correctly() {
        let mut registry = MarketRegistry::new();
        registry.add(create_binary_market("binary-1", "yes-1", "no-1"));
        registry.add(create_binary_market("binary-2", "yes-2", "no-2"));
        registry.add(create_multi_outcome_market(
            "multi",
            &[("a", "A"), ("b", "B"), ("c", "C")],
        ));

        let binary: Vec<_> = registry.binary_markets().collect();
        assert_eq!(binary.len(), 2);
        assert!(binary.iter().all(|m| m.is_binary()));
    }

    #[test]
    fn multi_outcome_markets_filters_correctly() {
        let mut registry = MarketRegistry::new();
        registry.add(create_binary_market("binary", "yes", "no"));
        registry.add(create_multi_outcome_market(
            "multi-3",
            &[("a", "A"), ("b", "B"), ("c", "C")],
        ));
        registry.add(create_multi_outcome_market(
            "multi-4",
            &[("w", "W"), ("x", "X"), ("y", "Y"), ("z", "Z")],
        ));

        let multi: Vec<_> = registry.multi_outcome_markets().collect();
        assert_eq!(multi.len(), 2);
        assert!(multi.iter().all(|m| m.outcome_count() >= 3));
    }

    #[test]
    fn all_token_ids_returns_all_tokens() {
        let mut registry = MarketRegistry::new();
        registry.add(create_binary_market("m1", "yes-1", "no-1"));
        registry.add(create_multi_outcome_market(
            "m2",
            &[("a", "A"), ("b", "B"), ("c", "C")],
        ));

        let token_ids: Vec<_> = registry.all_token_ids().collect();
        assert_eq!(token_ids.len(), 5); // 2 from binary + 3 from multi

        // Check specific tokens are present
        let token_strs: Vec<_> = token_ids.iter().map(|t| t.as_str()).collect();
        assert!(token_strs.contains(&"yes-1"));
        assert!(token_strs.contains(&"no-1"));
        assert!(token_strs.contains(&"a"));
        assert!(token_strs.contains(&"b"));
        assert!(token_strs.contains(&"c"));
    }

    #[test]
    fn len_and_is_empty() {
        let mut registry = MarketRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);

        registry.add(create_binary_market("m1", "yes", "no"));
        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);

        registry.add(create_binary_market("m2", "yes2", "no2"));
        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn add_multi_outcome_market_indexes_all_tokens() {
        let mut registry = MarketRegistry::new();
        registry.add(create_multi_outcome_market(
            "multi",
            &[
                ("red-token", "Red"),
                ("blue-token", "Blue"),
                ("green-token", "Green"),
                ("yellow-token", "Yellow"),
            ],
        ));

        assert_eq!(registry.len(), 1);

        // All 4 tokens should be indexed
        for token_id in &["red-token", "blue-token", "green-token", "yellow-token"] {
            let market = registry.get_by_token(&TokenId::from(*token_id));
            assert!(
                market.is_some(),
                "Token {} should be in registry",
                token_id
            );
            assert_eq!(market.unwrap().market_id().as_str(), "multi");
        }
    }
}
