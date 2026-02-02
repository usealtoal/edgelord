//! Polymarket-specific market registry for YES/NO pairs.
//!
//! This module provides a registry that maps token IDs to their associated
//! market pairs, enabling efficient lookup of market information from order
//! book events that only contain token IDs.

use std::collections::HashMap;

use super::types::Market;
use crate::domain::{MarketId, MarketPair, TokenId};

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

    /// Look up the market pair for a given token ID.
    ///
    /// Returns the complete market pair if the token is registered, or `None`
    /// if the token is not found in any registered market.
    pub fn get_market_for_token(&self, token_id: &TokenId) -> Option<&MarketPair> {
        self.token_to_market.get(token_id)
    }

    /// Get all registered market pairs.
    pub fn pairs(&self) -> &[MarketPair] {
        &self.pairs
    }

    /// Get the number of registered market pairs.
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }
}

impl Default for MarketRegistry {
    fn default() -> Self {
        Self::new()
    }
}
