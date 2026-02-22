//! Position tracking repository.

use rust_decimal::Decimal;

use crate::domain::{Position, PositionId, Price};

/// Tracks all open positions.
#[derive(Debug, Default)]
pub struct PositionTracker {
    positions: Vec<Position>,
    next_id: u64,
}

impl PositionTracker {
    /// Create a new position tracker.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            positions: Vec::new(),
            next_id: 1,
        }
    }

    /// Generate the next position ID and increment the counter.
    pub const fn next_id(&mut self) -> PositionId {
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
            .map(Position::entry_cost)
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

    /// Close a position and return the realized PnL.
    ///
    /// Returns None if position not found or already closed.
    pub fn close(&mut self, id: PositionId, pnl: Price) -> Option<Price> {
        let position = self.get_mut(id)?;
        if position.status().is_closed() {
            return None;
        }
        position.close(pnl);
        Some(pnl)
    }

    /// Get all positions (open and closed).
    pub fn all(&self) -> impl Iterator<Item = &Position> {
        self.positions.iter()
    }

    /// Get all closed positions.
    pub fn closed_positions(&self) -> impl Iterator<Item = &Position> {
        self.positions.iter().filter(|p| p.status().is_closed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{MarketId, PositionStatus};
    use rust_decimal_macros::dec;

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
        assert_eq!(tracker.total_exposure(), dec!(125));
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

        let position = tracker.get_mut(id).unwrap();
        position.close(dec!(5));

        assert_eq!(tracker.open_count(), 0);
        let position = tracker.get(id).unwrap();
        assert!(position.status().is_closed());
    }

    #[test]
    fn position_tracker_open_positions_iterator() {
        let mut tracker = PositionTracker::new();

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

        let open: Vec<_> = tracker.open_positions().collect();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].market_id().as_str(), "m1");
    }
}
