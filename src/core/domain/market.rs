//! Market-related domain types with proper encapsulation.
//!
//! This module provides exchange-agnostic market representations:
//!
//! - [`Market`] - A prediction market with N outcomes and configurable payout
//! - [`Outcome`] - A single tradeable outcome within a market
//!
//! These types work across any prediction market exchange, with exchange-specific
//! details (like payout amounts) configured at market creation time.

use rust_decimal::Decimal;
use std::result::Result;

use crate::error::DomainError;
use super::id::{MarketId, TokenId};

/// A single outcome within a market.
///
/// Each outcome has a unique token ID (used for trading) and a human-readable name.
/// For binary markets, typical names are "Yes"/"No". For multi-outcome markets,
/// names might be candidate names, team names, etc.
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

/// A prediction market supporting N outcomes with configurable payout.
///
/// This is the core market representation used throughout the system. It supports:
/// - **Binary markets** (2 outcomes, e.g., Yes/No)
/// - **Multi-outcome markets** (3+ outcomes, e.g., election candidates)
///
/// The `payout` field specifies how much one share pays out on resolution.
/// For Polymarket, this is $1.00. Other exchanges may use different values.
///
/// # Example
///
/// ```ignore
/// use edgelord::core::domain::{Market, Outcome, MarketId, TokenId};
/// use rust_decimal_macros::dec;
///
/// let market = Market::new(
///     MarketId::from("market-123"),
///     "Will it rain tomorrow?",
///     vec![
///         Outcome::new(TokenId::from("yes-token"), "Yes"),
///         Outcome::new(TokenId::from("no-token"), "No"),
///     ],
///     dec!(1.00), // $1 payout per share
/// );
/// ```
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

    /// Create a new market with domain invariant validation.
    ///
    /// # Domain Invariants
    ///
    /// - `outcomes` must not be empty
    /// - `payout` must be positive (> 0)
    ///
    /// # Errors
    ///
    /// Returns `DomainError` if any invariant is violated.
    pub fn try_new(
        market_id: MarketId,
        question: impl Into<String>,
        outcomes: Vec<Outcome>,
        payout: Decimal,
    ) -> Result<Self, DomainError> {
        // Validate outcomes is not empty
        if outcomes.is_empty() {
            return Err(DomainError::EmptyOutcomes);
        }

        // Validate payout is positive
        if payout <= Decimal::ZERO {
            return Err(DomainError::NonPositivePayout { payout });
        }

        Ok(Self {
            market_id,
            question: question.into(),
            outcomes,
            payout,
        })
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

    #[test]
    fn market_rejects_empty_outcomes() {
        // Empty outcomes should fail
        let result = Market::try_new(
            MarketId::from("market-1"),
            "Test question",
            vec![],
            dec!(1.00),
        );
        assert!(result.is_err());
    }

    #[test]
    fn market_rejects_non_positive_payout() {
        let outcomes = vec![
            Outcome::new(TokenId::from("yes-token"), "Yes"),
            Outcome::new(TokenId::from("no-token"), "No"),
        ];

        // Zero payout should fail
        let result = Market::try_new(
            MarketId::from("market-1"),
            "Test question",
            outcomes.clone(),
            dec!(0),
        );
        assert!(result.is_err());

        // Negative payout should fail
        let result = Market::try_new(
            MarketId::from("market-1"),
            "Test question",
            outcomes,
            dec!(-1),
        );
        assert!(result.is_err());
    }
}
