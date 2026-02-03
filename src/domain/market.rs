//! Market-related domain types with proper encapsulation.

use super::id::{MarketId, TokenId};

/// A YES/NO market pair with proper encapsulation.
#[derive(Debug, Clone)]
pub struct MarketPair {
    market_id: MarketId,
    question: String,
    yes_token: TokenId,
    no_token: TokenId,
}

impl MarketPair {
    /// Create a new `MarketPair`.
    pub fn new(
        market_id: MarketId,
        question: impl Into<String>,
        yes_token: TokenId,
        no_token: TokenId,
    ) -> Self {
        Self {
            market_id,
            question: question.into(),
            yes_token,
            no_token,
        }
    }

    /// Get the market ID.
    #[must_use]
    pub const fn market_id(&self) -> &MarketId {
        &self.market_id
    }

    /// Get the market question.
    #[must_use]
    pub fn question(&self) -> &str {
        &self.question
    }

    /// Get the YES token ID.
    #[must_use]
    pub const fn yes_token(&self) -> &TokenId {
        &self.yes_token
    }

    /// Get the NO token ID.
    #[must_use]
    pub const fn no_token(&self) -> &TokenId {
        &self.no_token
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn market_pair_accessors() {
        let pair = MarketPair::new(
            MarketId::from("market-1"),
            "Will it rain?",
            TokenId::from("yes-token"),
            TokenId::from("no-token"),
        );

        assert_eq!(pair.market_id().as_str(), "market-1");
        assert_eq!(pair.question(), "Will it rain?");
        assert_eq!(pair.yes_token().as_str(), "yes-token");
        assert_eq!(pair.no_token().as_str(), "no-token");
    }
}
