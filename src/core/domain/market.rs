//! Market-related domain types with proper encapsulation.

use rust_decimal::Decimal;

use super::id::{MarketId, TokenId};

/// A single outcome within a market.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    token_id: TokenId,
    name: String,
}

impl Outcome {
    /// Create a new outcome.
    pub fn new(token_id: TokenId, name: impl Into<String>) -> Self {
        Self {
            token_id,
            name: name.into(),
        }
    }

    /// Get the token ID for this outcome.
    #[must_use]
    pub const fn token_id(&self) -> &TokenId {
        &self.token_id
    }

    /// Get the name of this outcome.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// A generic market supporting N outcomes with configurable payout.
#[derive(Debug, Clone)]
pub struct Market {
    market_id: MarketId,
    question: String,
    outcomes: Vec<Outcome>,
    payout: Decimal,
}

impl Market {
    /// Create a new market.
    pub fn new(
        market_id: MarketId,
        question: impl Into<String>,
        outcomes: Vec<Outcome>,
        payout: Decimal,
    ) -> Self {
        Self {
            market_id,
            question: question.into(),
            outcomes,
            payout,
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

    /// Get the payout amount.
    #[must_use]
    pub const fn payout(&self) -> Decimal {
        self.payout
    }

    /// Get all outcomes.
    #[must_use]
    pub fn outcomes(&self) -> &[Outcome] {
        &self.outcomes
    }

    /// Check if this is a binary (YES/NO) market.
    #[must_use]
    pub fn is_binary(&self) -> bool {
        self.outcomes.len() == 2
    }

    /// Get the number of outcomes.
    #[must_use]
    pub fn outcome_count(&self) -> usize {
        self.outcomes.len()
    }

    /// Find an outcome by name (case-insensitive).
    #[must_use]
    pub fn outcome_by_name(&self, name: &str) -> Option<&Outcome> {
        let name_lower = name.to_lowercase();
        self.outcomes
            .iter()
            .find(|o| o.name.to_lowercase() == name_lower)
    }

    /// Get all token IDs in outcome order.
    #[must_use]
    pub fn token_ids(&self) -> Vec<&TokenId> {
        self.outcomes.iter().map(|o| &o.token_id).collect()
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
    use rust_decimal_macros::dec;

    // --- Market tests ---

    fn create_binary_market() -> Market {
        let outcomes = vec![
            Outcome::new(TokenId::from("yes-token"), "Yes"),
            Outcome::new(TokenId::from("no-token"), "No"),
        ];
        Market::new(
            MarketId::from("market-1"),
            "Will it rain tomorrow?",
            outcomes,
            dec!(1.00),
        )
    }

    fn create_multi_outcome_market() -> Market {
        let outcomes = vec![
            Outcome::new(TokenId::from("red-token"), "Red"),
            Outcome::new(TokenId::from("blue-token"), "Blue"),
            Outcome::new(TokenId::from("green-token"), "Green"),
        ];
        Market::new(
            MarketId::from("market-2"),
            "What color will win?",
            outcomes,
            dec!(1.00),
        )
    }

    #[test]
    fn outcome_new_and_accessors() {
        let outcome = Outcome::new(TokenId::from("token-123"), "Yes");
        assert_eq!(outcome.token_id().as_str(), "token-123");
        assert_eq!(outcome.name(), "Yes");
    }

    #[test]
    fn market_new_and_accessors() {
        let market = create_binary_market();
        assert_eq!(market.market_id().as_str(), "market-1");
        assert_eq!(market.question(), "Will it rain tomorrow?");
        assert_eq!(market.payout(), dec!(1.00));
        assert_eq!(market.outcomes().len(), 2);
    }

    #[test]
    fn is_binary_returns_true_for_two_outcomes() {
        let market = create_binary_market();
        assert!(market.is_binary());
    }

    #[test]
    fn is_binary_returns_false_for_more_than_two_outcomes() {
        let market = create_multi_outcome_market();
        assert!(!market.is_binary());
    }

    #[test]
    fn outcome_count_returns_correct_count() {
        let binary = create_binary_market();
        assert_eq!(binary.outcome_count(), 2);

        let multi = create_multi_outcome_market();
        assert_eq!(multi.outcome_count(), 3);
    }

    #[test]
    fn outcome_by_name_finds_exact_match() {
        let market = create_binary_market();
        let outcome = market.outcome_by_name("Yes");
        assert!(outcome.is_some());
        assert_eq!(outcome.unwrap().token_id().as_str(), "yes-token");
    }

    #[test]
    fn outcome_by_name_is_case_insensitive() {
        let market = create_binary_market();

        // lowercase
        let outcome = market.outcome_by_name("yes");
        assert!(outcome.is_some());
        assert_eq!(outcome.unwrap().name(), "Yes");

        // uppercase
        let outcome = market.outcome_by_name("YES");
        assert!(outcome.is_some());
        assert_eq!(outcome.unwrap().name(), "Yes");

        // mixed case
        let outcome = market.outcome_by_name("yEs");
        assert!(outcome.is_some());
        assert_eq!(outcome.unwrap().name(), "Yes");
    }

    #[test]
    fn outcome_by_name_returns_none_for_nonexistent() {
        let market = create_binary_market();
        let outcome = market.outcome_by_name("Maybe");
        assert!(outcome.is_none());
    }

    #[test]
    fn token_ids_returns_all_token_ids() {
        let market = create_binary_market();
        let ids = market.token_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.iter().any(|id| id.as_str() == "yes-token"));
        assert!(ids.iter().any(|id| id.as_str() == "no-token"));
    }

    #[test]
    fn token_ids_order_matches_outcomes() {
        let market = create_multi_outcome_market();
        let ids = market.token_ids();
        assert_eq!(ids[0].as_str(), "red-token");
        assert_eq!(ids[1].as_str(), "blue-token");
        assert_eq!(ids[2].as_str(), "green-token");
    }

    // --- MarketPair tests ---

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
