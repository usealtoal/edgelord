//! Position tracking.

#![allow(dead_code)]

use edgelord::domain::{MarketId, Price, TokenId, Volume};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

/// Unique position identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PositionId(pub u64);

/// Status of a position.
#[derive(Debug, Clone)]
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

/// A single leg of a position.
#[derive(Debug, Clone)]
pub struct PositionLeg {
    pub token_id: TokenId,
    pub size: Volume,
    pub entry_price: Price,
}

/// An arbitrage position (YES + NO tokens held).
#[derive(Debug, Clone)]
pub struct Position {
    pub id: PositionId,
    pub market_id: MarketId,
    pub legs: Vec<PositionLeg>,
    pub entry_cost: Price,
    pub guaranteed_payout: Price,
    pub opened_at: DateTime<Utc>,
    pub status: PositionStatus,
}

/// Tracks all open positions.
#[derive(Debug, Default)]
pub struct PositionTracker {
    positions: Vec<Position>,
    next_id: u64,
}

impl PositionTracker {
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            next_id: 1,
        }
    }

    /// Record a new position.
    pub fn add(&mut self, position: Position) {
        self.positions.push(position);
    }

    /// Get all open positions.
    pub fn open_positions(&self) -> Vec<&Position> {
        self.positions
            .iter()
            .filter(|p| matches!(p.status, PositionStatus::Open))
            .collect()
    }

    /// Total exposure (sum of entry costs for open positions).
    pub fn total_exposure(&self) -> Price {
        self.open_positions()
            .iter()
            .map(|p| p.entry_cost)
            .fold(Decimal::ZERO, |acc, cost| acc + cost)
    }

    /// Generate next position ID.
    pub fn next_id(&mut self) -> PositionId {
        let id = PositionId(self.next_id);
        self.next_id += 1;
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_position_tracker_new() {
        let tracker = PositionTracker::new();
        assert_eq!(tracker.open_positions().len(), 0);
        assert_eq!(tracker.total_exposure(), dec!(0));
    }

    #[test]
    fn test_position_id_increments() {
        let mut tracker = PositionTracker::new();

        let id1 = tracker.next_id();
        let id2 = tracker.next_id();
        let id3 = tracker.next_id();

        assert_eq!(id1.0, 1);
        assert_eq!(id2.0, 2);
        assert_eq!(id3.0, 3);
    }

    #[test]
    fn test_add_position() {
        let mut tracker = PositionTracker::new();

        let position = Position {
            id: tracker.next_id(),
            market_id: MarketId::from("market-1".to_string()),
            legs: vec![],
            entry_cost: dec!(95),
            guaranteed_payout: dec!(100),
            opened_at: chrono::Utc::now(),
            status: PositionStatus::Open,
        };

        tracker.add(position);

        assert_eq!(tracker.open_positions().len(), 1);
        assert_eq!(tracker.total_exposure(), dec!(95));
    }

    #[test]
    fn test_total_exposure_sums_open_positions() {
        let mut tracker = PositionTracker::new();

        // Add two open positions
        let id1 = tracker.next_id();
        tracker.add(Position {
            id: id1,
            market_id: MarketId::from("m1".to_string()),
            legs: vec![],
            entry_cost: dec!(50),
            guaranteed_payout: dec!(55),
            opened_at: chrono::Utc::now(),
            status: PositionStatus::Open,
        });

        let id2 = tracker.next_id();
        tracker.add(Position {
            id: id2,
            market_id: MarketId::from("m2".to_string()),
            legs: vec![],
            entry_cost: dec!(75),
            guaranteed_payout: dec!(80),
            opened_at: chrono::Utc::now(),
            status: PositionStatus::Open,
        });

        assert_eq!(tracker.open_positions().len(), 2);
        assert_eq!(tracker.total_exposure(), dec!(125)); // 50 + 75
    }

    #[test]
    fn test_closed_positions_not_in_exposure() {
        let mut tracker = PositionTracker::new();

        let id = tracker.next_id();
        tracker.add(Position {
            id,
            market_id: MarketId::from("m1".to_string()),
            legs: vec![],
            entry_cost: dec!(50),
            guaranteed_payout: dec!(55),
            opened_at: chrono::Utc::now(),
            status: PositionStatus::Closed { pnl: dec!(5) },
        });

        assert_eq!(tracker.open_positions().len(), 0);
        assert_eq!(tracker.total_exposure(), dec!(0));
    }
}
