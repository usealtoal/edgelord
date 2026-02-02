//! Position tracking types for exchange-agnostic position management.

use std::fmt;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

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
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the underlying value.
    #[must_use] 
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for PositionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "pos-{}", self.0)
    }
}

/// Status of a position.
#[derive(Debug, Clone, PartialEq)]
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
    pub fn is_open(&self) -> bool {
        matches!(self, PositionStatus::Open)
    }

    /// Returns true if the position is a partial fill.
    #[must_use] 
    pub fn is_partial(&self) -> bool {
        matches!(self, PositionStatus::PartialFill { .. })
    }

    /// Returns true if the position is closed.
    #[must_use] 
    pub fn is_closed(&self) -> bool {
        matches!(self, PositionStatus::Closed { .. })
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
    pub fn new(token_id: TokenId, size: Volume, entry_price: Price) -> Self {
        Self {
            token_id,
            size,
            entry_price,
        }
    }

    /// Get the token ID.
    #[must_use] 
    pub fn token_id(&self) -> &TokenId {
        &self.token_id
    }

    /// Get the size.
    #[must_use] 
    pub fn size(&self) -> Volume {
        self.size
    }

    /// Get the entry price.
    #[must_use] 
    pub fn entry_price(&self) -> Price {
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
}

impl Position {
    /// Create a new position.
    #[must_use] 
    pub fn new(
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
        }
    }

    /// Get the position ID.
    #[must_use] 
    pub fn id(&self) -> PositionId {
        self.id
    }

    /// Get the market ID.
    #[must_use] 
    pub fn market_id(&self) -> &MarketId {
        &self.market_id
    }

    /// Get the legs.
    #[must_use] 
    pub fn legs(&self) -> &[PositionLeg] {
        &self.legs
    }

    /// Get the entry cost.
    #[must_use] 
    pub fn entry_cost(&self) -> Price {
        self.entry_cost
    }

    /// Get the guaranteed payout.
    #[must_use] 
    pub fn guaranteed_payout(&self) -> Price {
        self.guaranteed_payout
    }

    /// Get when the position was opened.
    #[must_use] 
    pub fn opened_at(&self) -> DateTime<Utc> {
        self.opened_at
    }

    /// Get the current status.
    #[must_use] 
    pub fn status(&self) -> &PositionStatus {
        &self.status
    }

    /// Calculate the expected profit (`guaranteed_payout` - `entry_cost`).
    #[must_use] 
    pub fn expected_profit(&self) -> Price {
        self.guaranteed_payout - self.entry_cost
    }

    /// Returns true if the position is open.
    #[must_use] 
    pub fn is_open(&self) -> bool {
        self.status.is_open()
    }

    /// Close the position with the given `PnL`.
    pub fn close(&mut self, pnl: Price) {
        self.status = PositionStatus::Closed { pnl };
    }
}

/// Tracks all open positions.
#[derive(Debug, Default)]
pub struct PositionTracker {
    positions: Vec<Position>,
    next_id: u64,
}

impl PositionTracker {
    /// Create a new position tracker.
    #[must_use] 
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            next_id: 1,
        }
    }

    /// Generate the next position ID and increment the counter.
    pub fn next_id(&mut self) -> PositionId {
        let id = PositionId::new(self.next_id);
        self.next_id += 1;
        id
    }

    /// Record a new position.
    pub fn add(&mut self, position: Position) {
        self.positions.push(position);
    }

    /// Get an iterator over all open positions.
    pub fn open_positions(&self) -> impl Iterator<Item = &Position> {
        self.positions.iter().filter(|p| p.is_open())
    }

    /// Total exposure (sum of entry costs for open positions).
    #[must_use] 
    pub fn total_exposure(&self) -> Price {
        self.open_positions()
            .map(|p| p.entry_cost())
            .fold(Decimal::ZERO, |acc, cost| acc + cost)
    }

    /// Get the count of open positions.
    #[must_use] 
    pub fn open_count(&self) -> usize {
        self.open_positions().count()
    }

    /// Get a position by ID.
    #[must_use] 
    pub fn get(&self, id: PositionId) -> Option<&Position> {
        self.positions.iter().find(|p| p.id() == id)
    }

    /// Get a mutable reference to a position by ID.
    pub fn get_mut(&mut self, id: PositionId) -> Option<&mut Position> {
        self.positions.iter_mut().find(|p| p.id() == id)
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
    fn position_tracker_new() {
        let tracker = PositionTracker::new();
        assert_eq!(tracker.open_count(), 0);
        assert_eq!(tracker.total_exposure(), dec!(0));
    }

    #[test]
    fn position_tracker_next_id_increments() {
        let mut tracker = PositionTracker::new();

        let id1 = tracker.next_id();
        let id2 = tracker.next_id();
        let id3 = tracker.next_id();

        assert_eq!(id1.value(), 1);
        assert_eq!(id2.value(), 2);
        assert_eq!(id3.value(), 3);
    }

    #[test]
    fn position_tracker_add() {
        let mut tracker = PositionTracker::new();

        let position = Position::new(
            tracker.next_id(),
            MarketId::new("market-1"),
            vec![],
            dec!(95),
            dec!(100),
            chrono::Utc::now(),
            PositionStatus::Open,
        );

        tracker.add(position);

        assert_eq!(tracker.open_count(), 1);
        assert_eq!(tracker.total_exposure(), dec!(95));
    }

    #[test]
    fn position_tracker_total_exposure_sums_open_positions() {
        let mut tracker = PositionTracker::new();

        // Add two open positions
        let id1 = tracker.next_id();
        tracker.add(Position::new(
            id1,
            MarketId::new("m1"),
            vec![],
            dec!(50),
            dec!(55),
            chrono::Utc::now(),
            PositionStatus::Open,
        ));

        let id2 = tracker.next_id();
        tracker.add(Position::new(
            id2,
            MarketId::new("m2"),
            vec![],
            dec!(75),
            dec!(80),
            chrono::Utc::now(),
            PositionStatus::Open,
        ));

        assert_eq!(tracker.open_count(), 2);
        assert_eq!(tracker.total_exposure(), dec!(125)); // 50 + 75
    }

    #[test]
    fn position_tracker_closed_positions_not_in_exposure() {
        let mut tracker = PositionTracker::new();

        let id = tracker.next_id();
        tracker.add(Position::new(
            id,
            MarketId::new("m1"),
            vec![],
            dec!(50),
            dec!(55),
            chrono::Utc::now(),
            PositionStatus::Closed { pnl: dec!(5) },
        ));

        assert_eq!(tracker.open_count(), 0);
        assert_eq!(tracker.total_exposure(), dec!(0));
    }

    #[test]
    fn position_tracker_get() {
        let mut tracker = PositionTracker::new();

        let id = tracker.next_id();
        tracker.add(Position::new(
            id,
            MarketId::new("market-1"),
            vec![],
            dec!(95),
            dec!(100),
            chrono::Utc::now(),
            PositionStatus::Open,
        ));

        let position = tracker.get(id).unwrap();
        assert_eq!(position.market_id().as_str(), "market-1");

        // Non-existent ID
        assert!(tracker.get(PositionId::new(999)).is_none());
    }

    #[test]
    fn position_tracker_get_mut() {
        let mut tracker = PositionTracker::new();

        let id = tracker.next_id();
        tracker.add(Position::new(
            id,
            MarketId::new("market-1"),
            vec![],
            dec!(95),
            dec!(100),
            chrono::Utc::now(),
            PositionStatus::Open,
        ));

        // Close the position via mutable reference
        let position = tracker.get_mut(id).unwrap();
        position.close(dec!(5));

        // Verify it's closed now
        assert_eq!(tracker.open_count(), 0);
        let position = tracker.get(id).unwrap();
        assert!(position.status().is_closed());
    }

    #[test]
    fn position_tracker_open_positions_iterator() {
        let mut tracker = PositionTracker::new();

        // Add one open and one closed position
        let id1 = tracker.next_id();
        tracker.add(Position::new(
            id1,
            MarketId::new("m1"),
            vec![],
            dec!(50),
            dec!(55),
            chrono::Utc::now(),
            PositionStatus::Open,
        ));

        let id2 = tracker.next_id();
        tracker.add(Position::new(
            id2,
            MarketId::new("m2"),
            vec![],
            dec!(75),
            dec!(80),
            chrono::Utc::now(),
            PositionStatus::Closed { pnl: dec!(5) },
        ));

        // Only open positions should be returned
        let open: Vec<_> = tracker.open_positions().collect();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].market_id().as_str(), "m1");
    }
}
