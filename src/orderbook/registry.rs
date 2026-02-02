use std::collections::HashMap;

use crate::api::Market;
use crate::types::{MarketId, MarketPair, TokenId};

/// Registry mapping tokens to their market pairs
pub struct MarketRegistry {
    /// Token ID -> Market pair it belongs to
    token_to_market: HashMap<TokenId, MarketPair>,
    /// All market pairs
    pairs: Vec<MarketPair>,
}

impl MarketRegistry {
    pub fn new() -> Self {
        Self {
            token_to_market: HashMap::new(),
            pairs: Vec::new(),
        }
    }

    /// Build registry from API market data
    /// Only includes 2-outcome (YES/NO) markets
    pub fn from_markets(markets: &[Market]) -> Self {
        let mut registry = Self::new();

        for market in markets {
            // Only handle 2-outcome markets for single-condition arbitrage
            if market.tokens.len() != 2 {
                continue;
            }

            // Find YES and NO tokens
            let yes_token = market
                .tokens
                .iter()
                .find(|t| t.outcome.to_lowercase() == "yes");
            let no_token = market
                .tokens
                .iter()
                .find(|t| t.outcome.to_lowercase() == "no");

            if let (Some(yes), Some(no)) = (yes_token, no_token) {
                let pair = MarketPair {
                    market_id: MarketId::from(market.condition_id.clone()),
                    question: market.question.clone().unwrap_or_default(),
                    yes_token: TokenId::from(yes.token_id.clone()),
                    no_token: TokenId::from(no.token_id.clone()),
                };

                registry
                    .token_to_market
                    .insert(pair.yes_token.clone(), pair.clone());
                registry
                    .token_to_market
                    .insert(pair.no_token.clone(), pair.clone());
                registry.pairs.push(pair);
            }
        }

        registry
    }

    /// Get the market pair for a token
    pub fn get_market_for_token(&self, token_id: &TokenId) -> Option<&MarketPair> {
        self.token_to_market.get(token_id)
    }

    /// Get all market pairs
    pub fn pairs(&self) -> &[MarketPair] {
        &self.pairs
    }

    /// Number of registered pairs
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }
}

impl Default for MarketRegistry {
    fn default() -> Self {
        Self::new()
    }
}
