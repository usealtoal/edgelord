//! Market-related domain types with proper encapsulation.

use super::id::{MarketId, TokenId};

/// Information about a token in a market.
#[derive(Debug, Clone)]
pub struct TokenInfo {
    id: TokenId,
    outcome: String,
}

impl TokenInfo {
    /// Create a new TokenInfo.
    pub fn new(id: TokenId, outcome: impl Into<String>) -> Self {
        Self {
            id,
            outcome: outcome.into(),
        }
    }

    /// Get the token ID.
    pub fn id(&self) -> &TokenId {
        &self.id
    }

    /// Get the outcome description.
    pub fn outcome(&self) -> &str {
        &self.outcome
    }
}

/// Information about a market.
#[derive(Debug, Clone)]
pub struct MarketInfo {
    id: MarketId,
    question: String,
    tokens: Vec<TokenInfo>,
}

impl MarketInfo {
    /// Create a new MarketInfo.
    pub fn new(id: MarketId, question: impl Into<String>, tokens: Vec<TokenInfo>) -> Self {
        Self {
            id,
            question: question.into(),
            tokens,
        }
    }

    /// Get the market ID.
    pub fn id(&self) -> &MarketId {
        &self.id
    }

    /// Get the market question.
    pub fn question(&self) -> &str {
        &self.question
    }

    /// Get the tokens in this market.
    pub fn tokens(&self) -> &[TokenInfo] {
        &self.tokens
    }
}

/// A YES/NO market pair with proper encapsulation.
#[derive(Debug, Clone)]
pub struct MarketPair {
    market_id: MarketId,
    question: String,
    yes_token: TokenId,
    no_token: TokenId,
}

impl MarketPair {
    /// Create a new MarketPair.
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
    pub fn market_id(&self) -> &MarketId {
        &self.market_id
    }

    /// Get the market question.
    pub fn question(&self) -> &str {
        &self.question
    }

    /// Get the YES token ID.
    pub fn yes_token(&self) -> &TokenId {
        &self.yes_token
    }

    /// Get the NO token ID.
    pub fn no_token(&self) -> &TokenId {
        &self.no_token
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_info_accessors() {
        let info = TokenInfo::new(TokenId::from("token-1"), "Yes");
        assert_eq!(info.id().as_str(), "token-1");
        assert_eq!(info.outcome(), "Yes");
    }

    #[test]
    fn market_info_accessors() {
        let tokens = vec![
            TokenInfo::new(TokenId::from("yes"), "Yes"),
            TokenInfo::new(TokenId::from("no"), "No"),
        ];
        let info = MarketInfo::new(MarketId::from("market-1"), "Will it rain?", tokens);

        assert_eq!(info.id().as_str(), "market-1");
        assert_eq!(info.question(), "Will it rain?");
        assert_eq!(info.tokens().len(), 2);
    }

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
