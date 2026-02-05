//! Opportunity types for arbitrage detection.
//!
//! An [`Opportunity`] represents a detected arbitrage situation where buying
//! all outcomes costs less than the guaranteed payout. Each opportunity has
//! multiple [`OpportunityLeg`]s representing the individual purchases needed.

use rust_decimal::Decimal;
use std::result::Result;

use crate::error::DomainError;
use super::id::{MarketId, TokenId};
use super::money::Price;

/// A single leg of an opportunity representing one outcome to purchase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpportunityLeg {
    token_id: TokenId,
    ask_price: Price,
}

impl OpportunityLeg {
    /// Create a new opportunity leg.
    #[must_use]
    pub fn new(token_id: TokenId, ask_price: Price) -> Self {
        Self {
            token_id,
            ask_price,
        }
    }

    /// Get the token ID for this leg.
    #[must_use]
    pub fn token_id(&self) -> &TokenId {
        &self.token_id
    }

    /// Get the ask price for this leg.
    #[must_use]
    pub fn ask_price(&self) -> Price {
        self.ask_price
    }
}

/// An arbitrage opportunity supporting any number of outcomes.
///
/// Uses market-provided payout instead of assuming a hardcoded value.
///
/// Derived fields are calculated on access:
/// - `total_cost`: sum of all leg prices
/// - `edge`: payout - total_cost
/// - `expected_profit`: edge * volume
#[derive(Debug, Clone)]
pub struct Opportunity {
    market_id: MarketId,
    question: String,
    legs: Vec<OpportunityLeg>,
    volume: Decimal,
    payout: Decimal,
    strategy: String,
}

impl Opportunity {
    /// Create a new opportunity.
    #[must_use]
    pub fn new(
        market_id: MarketId,
        question: impl Into<String>,
        legs: Vec<OpportunityLeg>,
        volume: Decimal,
        payout: Decimal,
    ) -> Self {
        Self {
            market_id,
            question: question.into(),
            legs,
            volume,
            payout,
            strategy: String::new(),
        }
    }

    /// Create a new opportunity with strategy name.
    #[must_use]
    pub fn with_strategy(
        market_id: MarketId,
        question: impl Into<String>,
        legs: Vec<OpportunityLeg>,
        volume: Decimal,
        payout: Decimal,
        strategy: impl Into<String>,
    ) -> Self {
        Self {
            market_id,
            question: question.into(),
            legs,
            volume,
            payout,
            strategy: strategy.into(),
        }
    }

    /// Create a new opportunity with domain invariant validation.
    ///
    /// # Domain Invariants
    ///
    /// - `volume` must be positive (> 0)
    /// - `payout` must be greater than the total cost of all legs
    ///
    /// # Errors
    ///
    /// Returns `DomainError` if any invariant is violated.
    pub fn try_new(
        market_id: MarketId,
        question: impl Into<String>,
        legs: Vec<OpportunityLeg>,
        volume: Decimal,
        payout: Decimal,
    ) -> Result<Self, DomainError> {
        use rust_decimal::Decimal;
        use std::cmp::Ordering;

        // Validate volume is positive
        if volume <= Decimal::ZERO {
            return Err(DomainError::NonPositiveVolume { volume });
        }

        // Calculate total cost
        let total_cost: Decimal = legs.iter().map(|leg| leg.ask_price).sum();

        // Validate payout is greater than cost
        match payout.partial_cmp(&total_cost) {
            Some(Ordering::Greater) => {
                // Valid case
            }
            _ => {
                return Err(DomainError::PayoutNotGreaterThanCost {
                    payout,
                    cost: total_cost,
                });
            }
        }

        Ok(Self {
            market_id,
            question: question.into(),
            legs,
            volume,
            payout,
            strategy: String::new(),
        })
    }

    /// Get the strategy name.
    #[must_use]
    pub fn strategy(&self) -> &str {
        &self.strategy
    }

    /// Get the market ID.
    #[must_use]
    pub fn market_id(&self) -> &MarketId {
        &self.market_id
    }

    /// Get the market question.
    #[must_use]
    pub fn question(&self) -> &str {
        &self.question
    }

    /// Get the opportunity legs.
    #[must_use]
    pub fn legs(&self) -> &[OpportunityLeg] {
        &self.legs
    }

    /// Get the volume.
    #[must_use]
    pub fn volume(&self) -> Decimal {
        self.volume
    }

    /// Get the payout amount.
    #[must_use]
    pub fn payout(&self) -> Decimal {
        self.payout
    }

    /// Calculate the total cost (sum of all leg prices).
    #[must_use]
    pub fn total_cost(&self) -> Decimal {
        self.legs.iter().map(|leg| leg.ask_price).sum()
    }

    /// Calculate the edge (payout - total_cost).
    #[must_use]
    pub fn edge(&self) -> Decimal {
        self.payout - self.total_cost()
    }

    /// Calculate the expected profit (edge * volume).
    #[must_use]
    pub fn expected_profit(&self) -> Decimal {
        self.edge() * self.volume
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn make_market_id() -> MarketId {
        MarketId::from("test-market")
    }

    fn make_token_id(name: &str) -> TokenId {
        TokenId::from(name)
    }

    #[test]
    fn leg_stores_token_and_price() {
        let leg = OpportunityLeg::new(make_token_id("outcome-a"), dec!(0.45));

        assert_eq!(leg.token_id().as_str(), "outcome-a");
        assert_eq!(leg.ask_price(), dec!(0.45));
    }

    #[test]
    fn two_legs_calculates_total_cost() {
        let legs = vec![
            OpportunityLeg::new(make_token_id("yes"), dec!(0.40)),
            OpportunityLeg::new(make_token_id("no"), dec!(0.50)),
        ];

        let opp = Opportunity::new(
            make_market_id(),
            "Will it rain?",
            legs,
            dec!(100),
            dec!(1.0),
        );

        assert_eq!(opp.total_cost(), dec!(0.90));
    }

    #[test]
    fn edge_uses_payout_not_hardcoded_one() {
        let legs = vec![
            OpportunityLeg::new(make_token_id("yes"), dec!(0.40)),
            OpportunityLeg::new(make_token_id("no"), dec!(0.50)),
        ];

        let opp = Opportunity::new(
            make_market_id(),
            "Will it rain?",
            legs,
            dec!(100),
            dec!(1.0),
        );

        assert_eq!(opp.edge(), dec!(0.10));
    }

    #[test]
    fn expected_profit_is_edge_times_volume() {
        let legs = vec![
            OpportunityLeg::new(make_token_id("yes"), dec!(0.40)),
            OpportunityLeg::new(make_token_id("no"), dec!(0.50)),
        ];

        let opp = Opportunity::new(
            make_market_id(),
            "Will it rain?",
            legs,
            dec!(100),
            dec!(1.0),
        );

        assert_eq!(opp.expected_profit(), dec!(10.00));
    }

    #[test]
    fn custom_payout_affects_edge() {
        let legs = vec![
            OpportunityLeg::new(make_token_id("yes"), dec!(0.80)),
            OpportunityLeg::new(make_token_id("no"), dec!(1.00)),
        ];

        let opp = Opportunity::new(
            make_market_id(),
            "Special market",
            legs,
            dec!(50),
            dec!(2.0),
        );

        assert_eq!(opp.total_cost(), dec!(1.80));
        assert_eq!(opp.edge(), dec!(0.20));
        assert_eq!(opp.expected_profit(), dec!(10.00));
    }

    #[test]
    fn three_outcome_market() {
        let legs = vec![
            OpportunityLeg::new(make_token_id("candidate-a"), dec!(0.30)),
            OpportunityLeg::new(make_token_id("candidate-b"), dec!(0.35)),
            OpportunityLeg::new(make_token_id("candidate-c"), dec!(0.25)),
        ];

        let opp = Opportunity::new(
            make_market_id(),
            "Who will win?",
            legs,
            dec!(100),
            dec!(1.0),
        );

        assert_eq!(opp.total_cost(), dec!(0.90));
        assert_eq!(opp.edge(), dec!(0.10));
        assert_eq!(opp.expected_profit(), dec!(10.00));
    }

    #[test]
    fn negative_edge_when_overpriced() {
        let legs = vec![
            OpportunityLeg::new(make_token_id("yes"), dec!(0.60)),
            OpportunityLeg::new(make_token_id("no"), dec!(0.50)),
        ];

        let opp = Opportunity::new(make_market_id(), "Overpriced", legs, dec!(100), dec!(1.0));

        assert_eq!(opp.total_cost(), dec!(1.10));
        assert_eq!(opp.edge(), dec!(-0.10));
        assert_eq!(opp.expected_profit(), dec!(-10.00));
    }

    #[test]
    fn accessors_return_correct_values() {
        let legs = vec![
            OpportunityLeg::new(make_token_id("yes"), dec!(0.40)),
            OpportunityLeg::new(make_token_id("no"), dec!(0.50)),
        ];

        let opp = Opportunity::new(
            make_market_id(),
            "Will it rain?",
            legs,
            dec!(100),
            dec!(1.0),
        );

        assert_eq!(opp.market_id().as_str(), "test-market");
        assert_eq!(opp.question(), "Will it rain?");
        assert_eq!(opp.legs().len(), 2);
        assert_eq!(opp.volume(), dec!(100));
        assert_eq!(opp.payout(), dec!(1.0));
    }

    #[test]
    fn opportunity_rejects_non_positive_volume() {
        let legs = vec![
            OpportunityLeg::new(make_token_id("yes"), dec!(0.40)),
            OpportunityLeg::new(make_token_id("no"), dec!(0.50)),
        ];

        // Zero volume should fail
        let result = Opportunity::try_new(
            make_market_id(),
            "Test",
            legs.clone(),
            dec!(0),
            dec!(1.0),
        );
        assert!(result.is_err());

        // Negative volume should fail
        let result = Opportunity::try_new(
            make_market_id(),
            "Test",
            legs,
            dec!(-10),
            dec!(1.0),
        );
        assert!(result.is_err());
    }

    #[test]
    fn opportunity_rejects_payout_not_greater_than_cost() {
        let legs = vec![
            OpportunityLeg::new(make_token_id("yes"), dec!(0.40)),
            OpportunityLeg::new(make_token_id("no"), dec!(0.50)),
        ];
        // Total cost is 0.90, payout must be > 0.90

        // Payout equal to cost should fail
        let result = Opportunity::try_new(
            make_market_id(),
            "Test",
            legs.clone(),
            dec!(100),
            dec!(0.90),
        );
        assert!(result.is_err());

        // Payout less than cost should fail
        let result = Opportunity::try_new(
            make_market_id(),
            "Test",
            legs,
            dec!(100),
            dec!(0.80),
        );
        assert!(result.is_err());
    }
}
