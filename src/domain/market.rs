//! Market-related domain types.
//!
//! - [`Market`] - A prediction market with N outcomes and configurable payout
//! - [`Outcome`] - A single tradeable outcome within a market
//! - [`MarketRegistry`] - Index of markets by token ID and market ID

use std::collections::HashMap;
use std::result::Result;

use rust_decimal::Decimal;

use super::error::DomainError;
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
/// use edgelord::domain::{market::Market, market::Outcome, id::MarketId, id::TokenId};
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

/// Index of markets by token ID and market ID.
///
/// Enables efficient lookup from order book events.
#[derive(Debug, Default)]
pub struct MarketRegistry {
    token_to_market: HashMap<TokenId, Market>,
    market_id_to_market: HashMap<MarketId, Market>,
    markets: Vec<Market>,
}

impl MarketRegistry {
    /// Create an empty market registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            token_to_market: HashMap::new(),
            market_id_to_market: HashMap::new(),
            markets: Vec::new(),
        }
    }

    /// Add a market to the registry, indexing all its token IDs.
    pub fn add(&mut self, market: Market) {
        self.market_id_to_market
            .insert(market.market_id().clone(), market.clone());
        for outcome in market.outcomes() {
            self.token_to_market
                .insert(outcome.token_id().clone(), market.clone());
        }
        self.markets.push(market);
    }

    /// Look up a market by its market ID.
    #[must_use]
    pub fn get_by_market_id(&self, market_id: &MarketId) -> Option<&Market> {
        self.market_id_to_market.get(market_id)
    }

    /// Look up the market for a given token ID.
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
    fn market_try_new_accepts_valid_inputs() {
        let outcomes = vec![
            Outcome::new(TokenId::from("yes-token"), "Yes"),
            Outcome::new(TokenId::from("no-token"), "No"),
        ];

        let result = Market::try_new(
            MarketId::from("market-1"),
            "Test question",
            outcomes,
            dec!(1.00),
        );

        assert!(result.is_ok());
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

    // --- MarketRegistry tests ---

    fn create_binary_market_with_id(id: &str, yes_token: &str, no_token: &str) -> Market {
        let outcomes = vec![
            Outcome::new(TokenId::from(yes_token), "Yes"),
            Outcome::new(TokenId::from(no_token), "No"),
        ];
        Market::new(
            MarketId::from(id),
            format!("Market {id}?"),
            outcomes,
            dec!(1.00),
        )
    }

    #[test]
    fn registry_new_creates_empty() {
        let registry = MarketRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn registry_add_indexes_all_tokens() {
        let mut registry = MarketRegistry::new();
        registry.add(create_binary_market_with_id("m1", "yes-1", "no-1"));

        assert_eq!(registry.len(), 1);
        assert!(registry.get_by_token(&TokenId::from("yes-1")).is_some());
        assert!(registry.get_by_token(&TokenId::from("no-1")).is_some());
    }

    #[test]
    fn registry_get_by_market_id() {
        let mut registry = MarketRegistry::new();
        registry.add(create_binary_market_with_id("m1", "yes-1", "no-1"));

        let market = registry.get_by_market_id(&MarketId::from("m1"));
        assert!(market.is_some());
        assert_eq!(market.unwrap().market_id().as_str(), "m1");
    }

    #[test]
    fn registry_get_by_token_returns_correct_market() {
        let mut registry = MarketRegistry::new();
        registry.add(create_binary_market_with_id("m1", "yes-1", "no-1"));
        registry.add(create_binary_market_with_id("m2", "yes-2", "no-2"));

        let market = registry.get_by_token(&TokenId::from("yes-1")).unwrap();
        assert_eq!(market.market_id().as_str(), "m1");

        let market = registry.get_by_token(&TokenId::from("no-2")).unwrap();
        assert_eq!(market.market_id().as_str(), "m2");
    }

    #[test]
    fn registry_binary_markets_filters_correctly() {
        let mut registry = MarketRegistry::new();
        registry.add(create_binary_market_with_id("b1", "yes-1", "no-1"));
        registry.add(create_multi_outcome_market());

        let binary: Vec<_> = registry.binary_markets().collect();
        assert_eq!(binary.len(), 1);
        assert!(binary[0].is_binary());
    }
}
