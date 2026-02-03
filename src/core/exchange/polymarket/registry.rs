//! Market registry for YES/NO pairs.
//!
//! This module provides a registry that maps token IDs to their associated
//! market pairs, enabling efficient lookup of market information from order
//! book events that only contain token IDs.

use std::collections::HashMap;

use super::types::Market;
use crate::core::domain::{MarketId, MarketPair, TokenId};
use crate::core::exchange::MarketInfo;

/// Registry mapping tokens to their market pairs.
///
/// This is Polymarket-specific because it understands the YES/NO token structure.
/// The registry maintains bidirectional mappings: given a token ID, you can find
/// the complete market pair including both YES and NO tokens.
pub struct MarketRegistry {
    token_to_market: HashMap<TokenId, MarketPair>,
    pairs: Vec<MarketPair>,
}

impl MarketRegistry {
    /// Create an empty market registry.
    #[must_use] 
    pub fn new() -> Self {
        Self {
            token_to_market: HashMap::new(),
            pairs: Vec::new(),
        }
    }

    /// Build a registry from Polymarket API market data.
    ///
    /// Parses the market data and extracts YES/NO token pairs. Markets with
    /// more or fewer than 2 outcomes are skipped. Each token ID is mapped
    /// to its containing market pair for efficient lookup.
    #[must_use] 
    pub fn from_markets(markets: &[Market]) -> Self {
        let mut registry = Self::new();

        for market in markets {
            if market.tokens.len() != 2 {
                continue;
            }

            let yes_token = market
                .tokens
                .iter()
                .find(|t| t.outcome.to_lowercase() == "yes");
            let no_token = market
                .tokens
                .iter()
                .find(|t| t.outcome.to_lowercase() == "no");

            if let (Some(yes), Some(no)) = (yes_token, no_token) {
                let pair = MarketPair::new(
                    MarketId::from(market.condition_id.clone()),
                    market.question.clone().unwrap_or_default(),
                    TokenId::from(yes.token_id.clone()),
                    TokenId::from(no.token_id.clone()),
                );

                registry
                    .token_to_market
                    .insert(pair.yes_token().clone(), pair.clone());
                registry
                    .token_to_market
                    .insert(pair.no_token().clone(), pair.clone());
                registry.pairs.push(pair);
            }
        }

        registry
    }

    /// Build a registry from generic market info.
    ///
    /// Works with the exchange-agnostic `MarketInfo` type, extracting YES/NO pairs
    /// from binary markets.
    #[must_use]
    pub fn from_market_info(markets: &[MarketInfo]) -> Self {
        let mut registry = Self::new();

        for market in markets {
            if market.outcomes.len() != 2 {
                continue;
            }

            let yes = market
                .outcomes
                .iter()
                .find(|o| o.name.to_lowercase() == "yes");
            let no = market
                .outcomes
                .iter()
                .find(|o| o.name.to_lowercase() == "no");

            if let (Some(yes), Some(no)) = (yes, no) {
                let pair = MarketPair::new(
                    MarketId::from(market.id.clone()),
                    market.question.clone(),
                    TokenId::from(yes.token_id.clone()),
                    TokenId::from(no.token_id.clone()),
                );

                registry
                    .token_to_market
                    .insert(pair.yes_token().clone(), pair.clone());
                registry
                    .token_to_market
                    .insert(pair.no_token().clone(), pair.clone());
                registry.pairs.push(pair);
            }
        }

        registry
    }

    /// Look up the market pair for a given token ID.
    ///
    /// Returns the complete market pair if the token is registered, or `None`
    /// if the token is not found in any registered market.
    #[must_use] 
    pub fn get_market_for_token(&self, token_id: &TokenId) -> Option<&MarketPair> {
        self.token_to_market.get(token_id)
    }

    /// Get all registered market pairs.
    #[must_use] 
    pub fn pairs(&self) -> &[MarketPair] {
        &self.pairs
    }

    /// Get the number of registered market pairs.
    #[must_use] 
    pub const fn len(&self) -> usize {
        self.pairs.len()
    }

    /// Check if the registry is empty.
    #[must_use] 
    pub const fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }
}

impl Default for MarketRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::exchange::polymarket::types::Token;

    fn make_market(condition_id: &str, question: &str, yes_id: &str, no_id: &str) -> Market {
        Market {
            condition_id: condition_id.to_string(),
            question: Some(question.to_string()),
            tokens: vec![
                Token {
                    token_id: yes_id.to_string(),
                    outcome: "Yes".to_string(),
                    price: Some(0.5),
                },
                Token {
                    token_id: no_id.to_string(),
                    outcome: "No".to_string(),
                    price: Some(0.5),
                },
            ],
            active: true,
            closed: false,
        }
    }

    #[test]
    fn test_registry_new() {
        let registry = MarketRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_from_markets() {
        let markets = vec![
            make_market("cond-1", "Question 1?", "yes-1", "no-1"),
            make_market("cond-2", "Question 2?", "yes-2", "no-2"),
        ];

        let registry = MarketRegistry::from_markets(&markets);

        assert_eq!(registry.len(), 2);
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_registry_skips_non_binary() {
        let mut market = make_market("cond-1", "Multi?", "a", "b");
        market.tokens.push(Token {
            token_id: "c".to_string(),
            outcome: "Maybe".to_string(),
            price: None,
        });

        let registry = MarketRegistry::from_markets(&[market]);

        assert!(registry.is_empty());
    }

    #[test]
    fn test_get_market_for_token() {
        let markets = vec![make_market("cond-1", "Q?", "yes-1", "no-1")];
        let registry = MarketRegistry::from_markets(&markets);

        let yes_token = TokenId::from("yes-1");
        let no_token = TokenId::from("no-1");
        let unknown = TokenId::from("unknown");

        assert!(registry.get_market_for_token(&yes_token).is_some());
        assert!(registry.get_market_for_token(&no_token).is_some());
        assert!(registry.get_market_for_token(&unknown).is_none());

        // Both tokens map to same market
        let yes_market = registry.get_market_for_token(&yes_token).unwrap();
        let no_market = registry.get_market_for_token(&no_token).unwrap();
        assert_eq!(yes_market.market_id(), no_market.market_id());
    }
}
