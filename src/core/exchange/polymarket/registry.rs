//! Polymarket-specific registry for markets.
//!
//! This module provides a Polymarket-specific registry that maps token IDs to
//! their associated markets, enabling efficient lookup of market information
//! from order book events that only contain token IDs.
//!
//! For a generic registry that works with any market structure, see
//! `crate::core::domain::MarketRegistry`.

use std::collections::HashMap;

use super::config::POLYMARKET_PAYOUT;
use super::types::PolymarketMarket;
use crate::core::domain::{Market, MarketId, Outcome, TokenId};
use crate::core::exchange::MarketInfo;

/// Polymarket-specific registry mapping tokens to their markets.
///
/// This registry understands the token structure specific to Polymarket.
/// It maintains bidirectional mappings: given a token ID, you can find
/// the complete market including all outcome tokens.
///
/// For a generic registry that works with any market structure (including
/// multi-outcome markets), see `crate::core::domain::MarketRegistry`.
pub struct PolymarketRegistry {
    token_to_market: HashMap<TokenId, Market>,
    markets: Vec<Market>,
}

impl PolymarketRegistry {
    /// Create an empty market registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            token_to_market: HashMap::new(),
            markets: Vec::new(),
        }
    }

    /// Build a registry from Polymarket API market data.
    ///
    /// Parses the market data and extracts YES/NO token pairs. Markets with
    /// more or fewer than 2 outcomes are skipped. Each token ID is mapped
    /// to its containing market for efficient lookup.
    #[must_use]
    pub fn from_markets(polymarket_markets: &[PolymarketMarket]) -> Self {
        let mut registry = Self::new();

        for pm in polymarket_markets {
            if pm.tokens.len() != 2 {
                continue;
            }

            let yes_token = pm
                .tokens
                .iter()
                .find(|t| t.outcome.to_lowercase() == "yes");
            let no_token = pm
                .tokens
                .iter()
                .find(|t| t.outcome.to_lowercase() == "no");

            if let (Some(yes), Some(no)) = (yes_token, no_token) {
                let outcomes = vec![
                    Outcome::new(TokenId::from(yes.token_id.clone()), "Yes"),
                    Outcome::new(TokenId::from(no.token_id.clone()), "No"),
                ];
                let market = Market::new(
                    MarketId::from(pm.condition_id.clone()),
                    pm.question.clone().unwrap_or_default(),
                    outcomes,
                    POLYMARKET_PAYOUT,
                );

                // Map both token IDs to this market
                for outcome in market.outcomes() {
                    registry
                        .token_to_market
                        .insert(outcome.token_id().clone(), market.clone());
                }
                registry.markets.push(market);
            }
        }

        registry
    }

    /// Build a registry from generic market info.
    ///
    /// Works with the exchange-agnostic `MarketInfo` type, extracting YES/NO pairs
    /// from binary markets.
    #[must_use]
    pub fn from_market_info(market_infos: &[MarketInfo]) -> Self {
        let mut registry = Self::new();

        for mi in market_infos {
            if mi.outcomes.len() != 2 {
                continue;
            }

            let yes = mi
                .outcomes
                .iter()
                .find(|o| o.name.to_lowercase() == "yes");
            let no = mi
                .outcomes
                .iter()
                .find(|o| o.name.to_lowercase() == "no");

            if let (Some(yes), Some(no)) = (yes, no) {
                let outcomes = vec![
                    Outcome::new(TokenId::from(yes.token_id.clone()), "Yes"),
                    Outcome::new(TokenId::from(no.token_id.clone()), "No"),
                ];
                let market = Market::new(
                    MarketId::from(mi.id.clone()),
                    mi.question.clone(),
                    outcomes,
                    POLYMARKET_PAYOUT,
                );

                // Map both token IDs to this market
                for outcome in market.outcomes() {
                    registry
                        .token_to_market
                        .insert(outcome.token_id().clone(), market.clone());
                }
                registry.markets.push(market);
            }
        }

        registry
    }

    /// Look up the market for a given token ID.
    ///
    /// Returns the complete market if the token is registered, or `None`
    /// if the token is not found in any registered market.
    #[must_use]
    pub fn get_market_for_token(&self, token_id: &TokenId) -> Option<&Market> {
        self.token_to_market.get(token_id)
    }

    /// Get all registered markets.
    #[must_use]
    pub fn markets(&self) -> &[Market] {
        &self.markets
    }

    /// Get the number of registered markets.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.markets.len()
    }

    /// Check if the registry is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.markets.is_empty()
    }
}

impl Default for PolymarketRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::exchange::polymarket::types::PolymarketToken;

    fn make_market(condition_id: &str, question: &str, yes_id: &str, no_id: &str) -> PolymarketMarket {
        PolymarketMarket {
            condition_id: condition_id.to_string(),
            question: Some(question.to_string()),
            tokens: vec![
                PolymarketToken {
                    token_id: yes_id.to_string(),
                    outcome: "Yes".to_string(),
                    price: Some(0.5),
                },
                PolymarketToken {
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
        let registry = PolymarketRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_from_markets() {
        let markets = vec![
            make_market("cond-1", "Question 1?", "yes-1", "no-1"),
            make_market("cond-2", "Question 2?", "yes-2", "no-2"),
        ];

        let registry = PolymarketRegistry::from_markets(&markets);

        assert_eq!(registry.len(), 2);
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_registry_skips_non_binary() {
        let mut market = make_market("cond-1", "Multi?", "a", "b");
        market.tokens.push(PolymarketToken {
            token_id: "c".to_string(),
            outcome: "Maybe".to_string(),
            price: None,
        });

        let registry = PolymarketRegistry::from_markets(&[market]);

        assert!(registry.is_empty());
    }

    #[test]
    fn test_get_market_for_token() {
        let markets = vec![make_market("cond-1", "Q?", "yes-1", "no-1")];
        let registry = PolymarketRegistry::from_markets(&markets);

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
