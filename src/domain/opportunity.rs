//! Opportunity type with builder pattern.
//!
//! This module provides the `Opportunity` struct representing a detected
//! arbitrage opportunity, along with `OpportunityBuilder` for safe construction.

use rust_decimal::Decimal;
use std::fmt;

use super::ids::{MarketId, TokenId};
use super::money::{Price, Volume};

/// Error returned when building an Opportunity fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpportunityBuildError {
    /// Market ID is required but was not provided.
    MissingMarketId,
    /// Question is required but was not provided.
    MissingQuestion,
    /// YES token is required but was not provided.
    MissingYesToken,
    /// NO token is required but was not provided.
    MissingNoToken,
    /// Volume is required but was not provided.
    MissingVolume,
}

impl fmt::Display for OpportunityBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMarketId => write!(f, "market_id is required"),
            Self::MissingQuestion => write!(f, "question is required"),
            Self::MissingYesToken => write!(f, "yes_token and yes_ask are required"),
            Self::MissingNoToken => write!(f, "no_token and no_ask are required"),
            Self::MissingVolume => write!(f, "volume is required"),
        }
    }
}

impl std::error::Error for OpportunityBuildError {}

/// A detected arbitrage opportunity.
///
/// Use `Opportunity::builder()` to construct instances.
/// The builder calculates derived fields (total_cost, edge, expected_profit)
/// automatically.
#[derive(Debug, Clone)]
pub struct Opportunity {
    market_id: MarketId,
    question: String,
    yes_token: TokenId,
    no_token: TokenId,
    yes_ask: Price,
    no_ask: Price,
    total_cost: Price,
    edge: Price,
    volume: Volume,
    expected_profit: Price,
}

impl Opportunity {
    /// Create a new builder for constructing an Opportunity.
    pub fn builder() -> OpportunityBuilder {
        OpportunityBuilder::new()
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

    /// Get the YES ask price.
    pub fn yes_ask(&self) -> Price {
        self.yes_ask
    }

    /// Get the NO ask price.
    pub fn no_ask(&self) -> Price {
        self.no_ask
    }

    /// Get the total cost (yes_ask + no_ask).
    pub fn total_cost(&self) -> Price {
        self.total_cost
    }

    /// Get the edge (1.0 - total_cost).
    pub fn edge(&self) -> Price {
        self.edge
    }

    /// Get the volume.
    pub fn volume(&self) -> Volume {
        self.volume
    }

    /// Get the expected profit (edge * volume).
    pub fn expected_profit(&self) -> Price {
        self.expected_profit
    }
}

/// Builder for constructing `Opportunity` instances.
///
/// # Example
///
/// ```ignore
/// let opportunity = Opportunity::builder()
///     .market_id(market_id)
///     .question("Will X happen?")
///     .yes_token(yes_token, yes_price)
///     .no_token(no_token, no_price)
///     .volume(volume)
///     .build()?;
/// ```
#[derive(Debug, Default)]
pub struct OpportunityBuilder {
    market_id: Option<MarketId>,
    question: Option<String>,
    yes_token: Option<TokenId>,
    yes_ask: Option<Price>,
    no_token: Option<TokenId>,
    no_ask: Option<Price>,
    volume: Option<Volume>,
}

impl OpportunityBuilder {
    /// Create a new empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the market ID.
    pub fn market_id(mut self, market_id: MarketId) -> Self {
        self.market_id = Some(market_id);
        self
    }

    /// Set the market question.
    pub fn question(mut self, question: impl Into<String>) -> Self {
        self.question = Some(question.into());
        self
    }

    /// Set the YES token and its ask price.
    pub fn yes_token(mut self, token: TokenId, ask: Price) -> Self {
        self.yes_token = Some(token);
        self.yes_ask = Some(ask);
        self
    }

    /// Set the NO token and its ask price.
    pub fn no_token(mut self, token: TokenId, ask: Price) -> Self {
        self.no_token = Some(token);
        self.no_ask = Some(ask);
        self
    }

    /// Set the volume.
    pub fn volume(mut self, volume: Volume) -> Self {
        self.volume = Some(volume);
        self
    }

    /// Build the Opportunity, calculating derived fields.
    ///
    /// # Errors
    ///
    /// Returns `OpportunityBuildError` if any required field is missing.
    pub fn build(self) -> Result<Opportunity, OpportunityBuildError> {
        let market_id = self.market_id.ok_or(OpportunityBuildError::MissingMarketId)?;
        let question = self.question.ok_or(OpportunityBuildError::MissingQuestion)?;
        let yes_token = self.yes_token.ok_or(OpportunityBuildError::MissingYesToken)?;
        let yes_ask = self.yes_ask.ok_or(OpportunityBuildError::MissingYesToken)?;
        let no_token = self.no_token.ok_or(OpportunityBuildError::MissingNoToken)?;
        let no_ask = self.no_ask.ok_or(OpportunityBuildError::MissingNoToken)?;
        let volume = self.volume.ok_or(OpportunityBuildError::MissingVolume)?;

        // Calculate derived fields
        let total_cost = yes_ask + no_ask;
        let edge = Decimal::ONE - total_cost;
        let expected_profit = edge * volume;

        Ok(Opportunity {
            market_id,
            question,
            yes_token,
            no_token,
            yes_ask,
            no_ask,
            total_cost,
            edge,
            volume,
            expected_profit,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn make_market_id() -> MarketId {
        MarketId::from("test-market")
    }

    fn make_yes_token() -> TokenId {
        TokenId::from("yes-token")
    }

    fn make_no_token() -> TokenId {
        TokenId::from("no-token")
    }

    #[test]
    fn builder_creates_opportunity_with_calculated_fields() {
        let opp = Opportunity::builder()
            .market_id(make_market_id())
            .question("Will it rain?")
            .yes_token(make_yes_token(), dec!(0.40))
            .no_token(make_no_token(), dec!(0.50))
            .volume(dec!(100))
            .build()
            .unwrap();

        assert_eq!(opp.market_id().as_str(), "test-market");
        assert_eq!(opp.question(), "Will it rain?");
        assert_eq!(opp.yes_ask(), dec!(0.40));
        assert_eq!(opp.no_ask(), dec!(0.50));
        assert_eq!(opp.total_cost(), dec!(0.90));
        assert_eq!(opp.edge(), dec!(0.10));
        assert_eq!(opp.volume(), dec!(100));
        assert_eq!(opp.expected_profit(), dec!(10.00));
    }

    #[test]
    fn builder_fails_without_market_id() {
        let result = Opportunity::builder()
            .question("Will it rain?")
            .yes_token(make_yes_token(), dec!(0.40))
            .no_token(make_no_token(), dec!(0.50))
            .volume(dec!(100))
            .build();

        assert_eq!(result.unwrap_err(), OpportunityBuildError::MissingMarketId);
    }

    #[test]
    fn builder_fails_without_question() {
        let result = Opportunity::builder()
            .market_id(make_market_id())
            .yes_token(make_yes_token(), dec!(0.40))
            .no_token(make_no_token(), dec!(0.50))
            .volume(dec!(100))
            .build();

        assert_eq!(result.unwrap_err(), OpportunityBuildError::MissingQuestion);
    }

    #[test]
    fn builder_fails_without_yes_token() {
        let result = Opportunity::builder()
            .market_id(make_market_id())
            .question("Will it rain?")
            .no_token(make_no_token(), dec!(0.50))
            .volume(dec!(100))
            .build();

        assert_eq!(result.unwrap_err(), OpportunityBuildError::MissingYesToken);
    }

    #[test]
    fn builder_fails_without_no_token() {
        let result = Opportunity::builder()
            .market_id(make_market_id())
            .question("Will it rain?")
            .yes_token(make_yes_token(), dec!(0.40))
            .volume(dec!(100))
            .build();

        assert_eq!(result.unwrap_err(), OpportunityBuildError::MissingNoToken);
    }

    #[test]
    fn builder_fails_without_volume() {
        let result = Opportunity::builder()
            .market_id(make_market_id())
            .question("Will it rain?")
            .yes_token(make_yes_token(), dec!(0.40))
            .no_token(make_no_token(), dec!(0.50))
            .build();

        assert_eq!(result.unwrap_err(), OpportunityBuildError::MissingVolume);
    }

    #[test]
    fn error_display_messages() {
        assert_eq!(
            OpportunityBuildError::MissingMarketId.to_string(),
            "market_id is required"
        );
        assert_eq!(
            OpportunityBuildError::MissingQuestion.to_string(),
            "question is required"
        );
        assert_eq!(
            OpportunityBuildError::MissingYesToken.to_string(),
            "yes_token and yes_ask are required"
        );
        assert_eq!(
            OpportunityBuildError::MissingNoToken.to_string(),
            "no_token and no_ask are required"
        );
        assert_eq!(
            OpportunityBuildError::MissingVolume.to_string(),
            "volume is required"
        );
    }

    #[test]
    fn builder_calculates_negative_edge() {
        // When total_cost > 1, edge is negative
        let opp = Opportunity::builder()
            .market_id(make_market_id())
            .question("Will it rain?")
            .yes_token(make_yes_token(), dec!(0.60))
            .no_token(make_no_token(), dec!(0.50))
            .volume(dec!(100))
            .build()
            .unwrap();

        assert_eq!(opp.total_cost(), dec!(1.10));
        assert_eq!(opp.edge(), dec!(-0.10));
        assert_eq!(opp.expected_profit(), dec!(-10.00));
    }
}
