//! Position types for exchange-agnostic position management.

use std::fmt;
use std::result::Result;

use chrono::{DateTime, Utc};

use crate::error::DomainError;
use super::{MarketId, Price, TokenId, Volume};

/// Unique position identifier.
///
/// The inner u64 is private to ensure all construction goes through
/// the defined constructors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PositionId(u64);

impl PositionId {
    /// Create a new `PositionId` from a u64 value.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the underlying value.
    #[must_use]
    pub const fn value(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for PositionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "pos-{}", self.0)
    }
}

/// Status of a position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PositionStatus {
    /// All legs filled successfully.
    Open,
    /// Some legs filled, exposure exists.
    PartialFill {
        filled: Vec<TokenId>,
        missing: Vec<TokenId>,
    },
    /// Position closed (market settled or sold).
    Closed { pnl: Price },
}

impl PositionStatus {
    /// Returns true if the position is open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        matches!(self, Self::Open)
    }

    /// Returns true if the position is a partial fill.
    #[must_use]
    pub const fn is_partial(&self) -> bool {
        matches!(self, Self::PartialFill { .. })
    }

    /// Returns true if the position is closed.
    #[must_use]
    pub const fn is_closed(&self) -> bool {
        matches!(self, Self::Closed { .. })
    }
}

/// A single leg of a position.
#[derive(Debug, Clone)]
pub struct PositionLeg {
    token_id: TokenId,
    size: Volume,
    entry_price: Price,
}

impl PositionLeg {
    /// Create a new position leg.
    #[must_use]
    pub const fn new(token_id: TokenId, size: Volume, entry_price: Price) -> Self {
        Self {
            token_id,
            size,
            entry_price,
        }
    }

    /// Get the token ID.
    #[must_use]
    pub const fn token_id(&self) -> &TokenId {
        &self.token_id
    }

    /// Get the size.
    #[must_use]
    pub const fn size(&self) -> Volume {
        self.size
    }

    /// Get the entry price.
    #[must_use]
    pub const fn entry_price(&self) -> Price {
        self.entry_price
    }

    /// Calculate the cost of this leg (size * `entry_price`).
    #[must_use]
    pub fn cost(&self) -> Price {
        self.size * self.entry_price
    }
}

/// An arbitrage position (YES + NO tokens held).
#[derive(Debug, Clone)]
pub struct Position {
    id: PositionId,
    market_id: MarketId,
    legs: Vec<PositionLeg>,
    entry_cost: Price,
    guaranteed_payout: Price,
    opened_at: DateTime<Utc>,
    status: PositionStatus,
    /// Associated trade ID for stats tracking.
    trade_id: Option<i32>,
}

impl Position {
    /// Create a new position.
    #[must_use]
    pub const fn new(
        id: PositionId,
        market_id: MarketId,
        legs: Vec<PositionLeg>,
        entry_cost: Price,
        guaranteed_payout: Price,
        opened_at: DateTime<Utc>,
        status: PositionStatus,
    ) -> Self {
        Self {
            id,
            market_id,
            legs,
            entry_cost,
            guaranteed_payout,
            opened_at,
            status,
            trade_id: None,
        }
    }

    /// Create a new position with an associated trade ID.
    #[must_use]
    pub const fn with_trade_id(mut self, trade_id: i32) -> Self {
        self.trade_id = Some(trade_id);
        self
    }

    /// Create a new position with domain invariant validation.
    ///
    /// # Domain Invariants
    ///
    /// - `legs` must not be empty
    /// - `guaranteed_payout` must be greater than `entry_cost`
    ///
    /// # Errors
    ///
    /// Returns `DomainError` if any invariant is violated.
    pub fn try_new(
        id: PositionId,
        market_id: MarketId,
        legs: Vec<PositionLeg>,
        entry_cost: Price,
        guaranteed_payout: Price,
        opened_at: DateTime<Utc>,
        status: PositionStatus,
    ) -> Result<Self, DomainError> {
        use std::cmp::Ordering;

        // Validate legs is not empty
        if legs.is_empty() {
            return Err(DomainError::EmptyLegs);
        }

        // Validate guaranteed_payout is greater than entry_cost
        match guaranteed_payout.partial_cmp(&entry_cost) {
            Some(Ordering::Greater) => {
                // Valid case
            }
            _ => {
                return Err(DomainError::PayoutNotGreaterThanCost {
                    payout: guaranteed_payout,
                    cost: entry_cost,
                });
            }
        }

        Ok(Self {
            id,
            market_id,
            legs,
            entry_cost,
            guaranteed_payout,
            opened_at,
            status,
            trade_id: None,
        })
    }

    /// Get the position ID.
    #[must_use]
    pub const fn id(&self) -> PositionId {
        self.id
    }

    /// Get the market ID.
    #[must_use]
    pub const fn market_id(&self) -> &MarketId {
        &self.market_id
    }

    /// Get the legs.
    #[must_use]
    pub fn legs(&self) -> &[PositionLeg] {
        &self.legs
    }

    /// Get the entry cost.
    #[must_use]
    pub const fn entry_cost(&self) -> Price {
        self.entry_cost
    }

    /// Get the guaranteed payout.
    #[must_use]
    pub const fn guaranteed_payout(&self) -> Price {
        self.guaranteed_payout
    }

    /// Get when the position was opened.
    #[must_use]
    pub const fn opened_at(&self) -> DateTime<Utc> {
        self.opened_at
    }

    /// Get the current status.
    #[must_use]
    pub const fn status(&self) -> &PositionStatus {
        &self.status
    }

    /// Get the associated trade ID (for stats tracking).
    #[must_use]
    pub const fn trade_id(&self) -> Option<i32> {
        self.trade_id
    }

    /// Calculate the expected profit (`guaranteed_payout` - `entry_cost`).
    #[must_use]
    pub fn expected_profit(&self) -> Price {
        self.guaranteed_payout - self.entry_cost
    }

    /// Returns true if the position is open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.status.is_open()
    }

    /// Close the position with the given `PnL`.
    pub fn close(&mut self, pnl: Price) {
        self.status = PositionStatus::Closed { pnl };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn position_id_new_and_value() {
        let id = PositionId::new(42);
        assert_eq!(id.value(), 42);
    }

    #[test]
    fn position_id_display() {
        let id = PositionId::new(123);
        assert_eq!(format!("{}", id), "pos-123");
    }

    #[test]
    fn position_status_is_open() {
        let open = PositionStatus::Open;
        let partial = PositionStatus::PartialFill {
            filled: vec![],
            missing: vec![],
        };
        let closed = PositionStatus::Closed { pnl: dec!(5) };

        assert!(open.is_open());
        assert!(!partial.is_open());
        assert!(!closed.is_open());
    }

    #[test]
    fn position_status_is_partial() {
        let open = PositionStatus::Open;
        let partial = PositionStatus::PartialFill {
            filled: vec![],
            missing: vec![],
        };
        let closed = PositionStatus::Closed { pnl: dec!(5) };

        assert!(!open.is_partial());
        assert!(partial.is_partial());
        assert!(!closed.is_partial());
    }

    #[test]
    fn position_status_is_closed() {
        let open = PositionStatus::Open;
        let partial = PositionStatus::PartialFill {
            filled: vec![],
            missing: vec![],
        };
        let closed = PositionStatus::Closed { pnl: dec!(5) };

        assert!(!open.is_closed());
        assert!(!partial.is_closed());
        assert!(closed.is_closed());
    }

    #[test]
    fn position_leg_new_and_accessors() {
        let leg = PositionLeg::new(TokenId::new("token-1"), dec!(100), dec!(0.45));

        assert_eq!(leg.token_id().as_str(), "token-1");
        assert_eq!(leg.size(), dec!(100));
        assert_eq!(leg.entry_price(), dec!(0.45));
    }

    #[test]
    fn position_leg_cost() {
        let leg = PositionLeg::new(TokenId::new("token-1"), dec!(100), dec!(0.45));
        assert_eq!(leg.cost(), dec!(45)); // 100 * 0.45
    }

    #[test]
    fn position_new_and_accessors() {
        let now = chrono::Utc::now();
        let position = Position::new(
            PositionId::new(1),
            MarketId::new("market-1"),
            vec![PositionLeg::new(
                TokenId::new("token-1"),
                dec!(100),
                dec!(0.45),
            )],
            dec!(95),
            dec!(100),
            now,
            PositionStatus::Open,
        );

        assert_eq!(position.id().value(), 1);
        assert_eq!(position.market_id().as_str(), "market-1");
        assert_eq!(position.legs().len(), 1);
        assert_eq!(position.entry_cost(), dec!(95));
        assert_eq!(position.guaranteed_payout(), dec!(100));
        assert_eq!(position.opened_at(), now);
        assert!(position.status().is_open());
    }

    #[test]
    fn position_expected_profit() {
        let position = Position::new(
            PositionId::new(1),
            MarketId::new("market-1"),
            vec![],
            dec!(95),
            dec!(100),
            chrono::Utc::now(),
            PositionStatus::Open,
        );

        assert_eq!(position.expected_profit(), dec!(5)); // 100 - 95
    }

    #[test]
    fn position_is_open() {
        let open = Position::new(
            PositionId::new(1),
            MarketId::new("market-1"),
            vec![],
            dec!(95),
            dec!(100),
            chrono::Utc::now(),
            PositionStatus::Open,
        );

        let closed = Position::new(
            PositionId::new(2),
            MarketId::new("market-2"),
            vec![],
            dec!(95),
            dec!(100),
            chrono::Utc::now(),
            PositionStatus::Closed { pnl: dec!(5) },
        );

        assert!(open.is_open());
        assert!(!closed.is_open());
    }

    #[test]
    fn position_close() {
        let mut position = Position::new(
            PositionId::new(1),
            MarketId::new("market-1"),
            vec![],
            dec!(95),
            dec!(100),
            chrono::Utc::now(),
            PositionStatus::Open,
        );

        assert!(position.is_open());
        position.close(dec!(5));
        assert!(!position.is_open());
        assert!(position.status().is_closed());
    }

    #[test]
    fn position_rejects_payout_not_greater_than_cost() {
        let legs = vec![PositionLeg::new(
            TokenId::new("token-1"),
            dec!(100),
            dec!(0.45),
        )];

        // Payout equal to cost should fail
        let result = Position::try_new(
            PositionId::new(1),
            MarketId::new("market-1"),
            legs.clone(),
            dec!(45),
            dec!(45),
            chrono::Utc::now(),
            PositionStatus::Open,
        );
        assert!(result.is_err());

        // Payout less than cost should fail
        let result = Position::try_new(
            PositionId::new(1),
            MarketId::new("market-1"),
            legs,
            dec!(50),
            dec!(45),
            chrono::Utc::now(),
            PositionStatus::Open,
        );
        assert!(result.is_err());
    }

    #[test]
    fn position_rejects_empty_legs() {
        // Empty legs should fail
        let result = Position::try_new(
            PositionId::new(1),
            MarketId::new("market-1"),
            vec![],
            dec!(95),
            dec!(100),
            chrono::Utc::now(),
            PositionStatus::Open,
        );
        assert!(result.is_err());
    }
}
