//! Position types for exchange-agnostic position management.
//!
//! This module provides types for tracking open and closed arbitrage positions.
//! A position represents ownership of outcome shares across one or more legs
//! of an arbitrage trade.
//!
//! # Position Lifecycle
//!
//! 1. **Open**: All legs filled successfully, waiting for market resolution
//! 2. **Partial Fill**: Some legs filled, creating risk exposure
//! 3. **Closed**: Position exited via market settlement or sale
//!
//! # Examples
//!
//! Creating an arbitrage position:
//!
//! ```
//! use edgelord::domain::position::{Position, PositionLeg, PositionStatus};
//! use edgelord::domain::id::{PositionId, MarketId, TokenId};
//! use rust_decimal_macros::dec;
//! use chrono::Utc;
//!
//! let leg = PositionLeg::new(TokenId::new("yes-token"), dec!(100), dec!(0.45));
//! let position = Position::new(
//!     PositionId::new(1),
//!     MarketId::new("market-1"),
//!     vec![leg],
//!     dec!(95),   // entry cost
//!     dec!(100),  // guaranteed payout
//!     Utc::now(),
//!     PositionStatus::Open,
//! );
//!
//! assert_eq!(position.expected_profit(), dec!(5));
//! ```

use std::result::Result;

use chrono::{DateTime, Utc};

use super::error::DomainError;
use super::id::{MarketId, PositionId, TokenId};
use super::money::{Price, Volume};

/// Status of a trading position in its lifecycle.
///
/// Positions progress through states based on fill status and market events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PositionStatus {
    /// All legs filled successfully, position is active.
    Open,
    /// Some legs filled but not all, creating directional exposure.
    PartialFill {
        /// Token IDs of legs that were successfully filled.
        filled: Vec<TokenId>,
        /// Token IDs of legs that failed to fill.
        missing: Vec<TokenId>,
    },
    /// Position has been closed (market settled or shares sold).
    Closed {
        /// Realized profit or loss from this position.
        pnl: Price,
    },
}

impl PositionStatus {
    /// Returns true if the position is open and active.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        matches!(self, Self::Open)
    }

    /// Returns true if the position has partial fill exposure.
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

/// A single leg of a multi-leg position.
///
/// Each leg represents ownership of shares in one outcome of a market.
/// For arbitrage positions, multiple legs combine to create a hedged position.
///
/// # Examples
///
/// ```
/// use edgelord::domain::position::PositionLeg;
/// use edgelord::domain::id::TokenId;
/// use rust_decimal_macros::dec;
///
/// let leg = PositionLeg::new(TokenId::new("yes-token"), dec!(100), dec!(0.45));
///
/// assert_eq!(leg.size(), dec!(100));
/// assert_eq!(leg.entry_price(), dec!(0.45));
/// assert_eq!(leg.cost(), dec!(45)); // 100 * 0.45
/// ```
#[derive(Debug, Clone)]
pub struct PositionLeg {
    /// The token ID for this leg's outcome.
    token_id: TokenId,
    /// Number of shares held.
    size: Volume,
    /// Price paid per share.
    entry_price: Price,
}

impl PositionLeg {
    /// Creates a new position leg.
    #[must_use]
    pub const fn new(token_id: TokenId, size: Volume, entry_price: Price) -> Self {
        Self {
            token_id,
            size,
            entry_price,
        }
    }

    /// Returns the token ID for this leg.
    #[must_use]
    pub const fn token_id(&self) -> &TokenId {
        &self.token_id
    }

    /// Returns the number of shares held.
    #[must_use]
    pub const fn size(&self) -> Volume {
        self.size
    }

    /// Returns the entry price per share.
    #[must_use]
    pub const fn entry_price(&self) -> Price {
        self.entry_price
    }

    /// Calculates the total cost of this leg (size times entry price).
    #[must_use]
    pub fn cost(&self) -> Price {
        self.size * self.entry_price
    }
}

/// An arbitrage position holding shares across multiple outcomes.
///
/// A position represents the combined holdings from an arbitrage trade.
/// For a simple binary arbitrage, this includes both YES and NO shares
/// that together guarantee a profit regardless of outcome.
///
/// # Examples
///
/// ```
/// use edgelord::domain::position::{Position, PositionLeg, PositionStatus};
/// use edgelord::domain::id::{PositionId, MarketId, TokenId};
/// use rust_decimal_macros::dec;
/// use chrono::Utc;
///
/// let legs = vec![
///     PositionLeg::new(TokenId::new("yes"), dec!(100), dec!(0.45)),
///     PositionLeg::new(TokenId::new("no"), dec!(100), dec!(0.50)),
/// ];
///
/// let position = Position::new(
///     PositionId::new(1),
///     MarketId::new("market-1"),
///     legs,
///     dec!(95),   // paid $95 total
///     dec!(100),  // will receive $100 on any outcome
///     Utc::now(),
///     PositionStatus::Open,
/// );
///
/// assert_eq!(position.expected_profit(), dec!(5));
/// assert!(position.is_open());
/// ```
#[derive(Debug, Clone)]
pub struct Position {
    /// Unique identifier for this position.
    id: PositionId,
    /// The market this position is in.
    market_id: MarketId,
    /// Individual legs (outcome holdings) of this position.
    legs: Vec<PositionLeg>,
    /// Total cost paid to open this position.
    entry_cost: Price,
    /// Guaranteed payout on market resolution.
    guaranteed_payout: Price,
    /// Timestamp when the position was opened.
    opened_at: DateTime<Utc>,
    /// Current status of the position.
    status: PositionStatus,
    /// Associated trade ID for statistics tracking.
    trade_id: Option<i32>,
}

impl Position {
    /// Creates a new position without validation.
    ///
    /// Use [`Position::try_new`] for validated construction.
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

    /// Associates a trade ID with this position for statistics tracking.
    #[must_use]
    pub const fn with_trade_id(mut self, trade_id: i32) -> Self {
        self.trade_id = Some(trade_id);
        self
    }

    /// Creates a new position with domain invariant validation.
    ///
    /// # Domain Invariants
    ///
    /// - `legs` must not be empty
    /// - `guaranteed_payout` must be greater than `entry_cost`
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::EmptyLegs`] if legs is empty.
    /// Returns [`DomainError::PayoutNotGreaterThanCost`] if the payout
    /// does not exceed the entry cost.
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

        if legs.is_empty() {
            return Err(DomainError::EmptyLegs);
        }

        match guaranteed_payout.partial_cmp(&entry_cost) {
            Some(Ordering::Greater) => {}
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

    /// Returns the position ID.
    #[must_use]
    pub const fn id(&self) -> PositionId {
        self.id
    }

    /// Returns the market ID.
    #[must_use]
    pub const fn market_id(&self) -> &MarketId {
        &self.market_id
    }

    /// Returns all legs of this position.
    #[must_use]
    pub fn legs(&self) -> &[PositionLeg] {
        &self.legs
    }

    /// Returns the total entry cost for this position.
    #[must_use]
    pub const fn entry_cost(&self) -> Price {
        self.entry_cost
    }

    /// Returns the guaranteed payout on resolution.
    #[must_use]
    pub const fn guaranteed_payout(&self) -> Price {
        self.guaranteed_payout
    }

    /// Returns when this position was opened.
    #[must_use]
    pub const fn opened_at(&self) -> DateTime<Utc> {
        self.opened_at
    }

    /// Returns the current status of this position.
    #[must_use]
    pub const fn status(&self) -> &PositionStatus {
        &self.status
    }

    /// Returns the associated trade ID if set.
    #[must_use]
    pub const fn trade_id(&self) -> Option<i32> {
        self.trade_id
    }

    /// Calculates the expected profit (guaranteed payout minus entry cost).
    #[must_use]
    pub fn expected_profit(&self) -> Price {
        self.guaranteed_payout - self.entry_cost
    }

    /// Returns true if this position is open and active.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.status.is_open()
    }

    /// Closes this position with the realized profit or loss.
    pub fn close(&mut self, pnl: Price) {
        self.status = PositionStatus::Closed { pnl };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

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
    fn position_try_new_accepts_valid_inputs() {
        let legs = vec![PositionLeg::new(
            TokenId::new("token-1"),
            dec!(100),
            dec!(0.45),
        )];

        let result = Position::try_new(
            PositionId::new(1),
            MarketId::new("market-1"),
            legs,
            dec!(45),
            dec!(100),
            chrono::Utc::now(),
            PositionStatus::Open,
        );

        assert!(result.is_ok());
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
