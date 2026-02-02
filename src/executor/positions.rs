//! Position tracking.

#![allow(dead_code)]

use crate::domain::{MarketId, Price, TokenId, Volume};
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
