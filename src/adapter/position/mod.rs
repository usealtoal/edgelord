//! Position lifecycle management service.
//!
//! Handles position state transitions from open → closed, tracking
//! settlement events and recording stats.

use std::sync::Arc;

use rust_decimal::Decimal;
use tracing::{debug, info};

use crate::adapter::statistics::{StatsRecorder, TradeCloseEvent};
use crate::domain::{MarketId, Position, PositionId, Price};
use crate::runtime::cache::PositionTracker;

/// Reason for closing a position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloseReason {
    /// Market settled with a winning outcome.
    Settlement { winning_outcome: String },
    /// Manual exit before settlement.
    ManualExit,
    /// Stop loss triggered.
    StopLoss { trigger_price: Decimal },
    /// Take profit target reached.
    TakeProfit { trigger_price: Decimal },
    /// Position expired (time limit).
    Expired,
    /// System shutdown or error recovery.
    SystemExit { reason: String },
}

impl std::fmt::Display for CloseReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Settlement { winning_outcome } => {
                write!(f, "settlement:{winning_outcome}")
            }
            Self::ManualExit => write!(f, "manual_exit"),
            Self::StopLoss { trigger_price } => write!(f, "stop_loss:{trigger_price}"),
            Self::TakeProfit { trigger_price } => write!(f, "take_profit:{trigger_price}"),
            Self::Expired => write!(f, "expired"),
            Self::SystemExit { reason } => write!(f, "system_exit:{reason}"),
        }
    }
}

/// Result of a position close operation.
#[derive(Debug, Clone)]
pub struct CloseResult {
    /// Position ID that was closed.
    pub position_id: PositionId,
    /// Realized profit/loss.
    pub realized_pnl: Price,
    /// Reason for closing.
    pub reason: CloseReason,
}

/// Manages position lifecycle and integrates with stats.
pub struct PositionManager {
    stats: Arc<StatsRecorder>,
}

impl PositionManager {
    /// Create a new position manager.
    #[must_use]
    pub fn new(stats: Arc<StatsRecorder>) -> Self {
        Self { stats }
    }

    /// Close a position by ID with the given PnL and reason.
    ///
    /// Updates the position tracker, records stats, and returns the close result.
    pub fn close_position(
        &self,
        tracker: &mut PositionTracker,
        position_id: PositionId,
        realized_pnl: Price,
        reason: CloseReason,
        trade_id: Option<i32>,
    ) -> Option<CloseResult> {
        // Get position info before closing
        let position = tracker.get(position_id)?;
        if position.status().is_closed() {
            debug!(position_id = %position_id, "Position already closed");
            return None;
        }

        let market_id = position.market_id().clone();

        // Close in tracker
        tracker.close(position_id, realized_pnl)?;

        info!(
            position_id = %position_id,
            market_id = %market_id,
            pnl = %realized_pnl,
            reason = %reason,
            "Position closed"
        );

        // Record in stats
        if let Some(tid) = trade_id {
            self.stats.record_trade_close(&TradeCloseEvent {
                trade_id: tid,
                realized_profit: realized_pnl,
                reason: reason.to_string(),
            });
        }

        Some(CloseResult {
            position_id,
            realized_pnl,
            reason,
        })
    }

    /// Close all positions for a market (e.g., on settlement).
    ///
    /// Returns the total realized PnL across all closed positions.
    pub fn close_all_for_market(
        &self,
        tracker: &mut PositionTracker,
        market_id: &MarketId,
        pnl_calculator: impl Fn(&Position) -> Price,
        reason: CloseReason,
    ) -> Price {
        let mut total_pnl = Decimal::ZERO;

        // Find all open positions for this market with their trade IDs
        let position_info: Vec<(PositionId, Option<i32>)> = tracker
            .all()
            .filter(|p| p.market_id() == market_id && !p.status().is_closed())
            .map(|p| (p.id(), p.trade_id()))
            .collect();

        for (pos_id, trade_id) in position_info {
            if let Some(position) = tracker.get(pos_id) {
                let pnl = pnl_calculator(position);
                if let Some(result) =
                    self.close_position(tracker, pos_id, pnl, reason.clone(), trade_id)
                {
                    total_pnl += result.realized_pnl;
                }
            }
        }

        total_pnl
    }

    /// Calculate settlement PnL for an arbitrage position.
    ///
    /// For arbitrage, we hold all outcomes so we always get the payout
    /// minus our entry cost.
    #[must_use]
    pub fn calculate_arbitrage_pnl(position: &Position, payout_per_share: Decimal) -> Price {
        // Arbitrage position: we hold all outcomes
        // PnL = (payout × shares) - entry_cost
        let shares = position.guaranteed_payout(); // This is actually volume
        (payout_per_share * shares) - position.entry_cost()
    }
}

/// Event indicating a market has settled.
#[derive(Debug, Clone)]
pub struct MarketSettledEvent {
    /// Market that settled.
    pub market_id: MarketId,
    /// Which outcome won (e.g., "Yes", "No", outcome name).
    pub winning_outcome: String,
    /// Payout amount per winning share.
    pub payout_per_share: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{PositionLeg, PositionStatus, TokenId};
    use chrono::Utc;
    use rust_decimal_macros::dec;

    fn make_position(id: u64, market_id: &str, entry_cost: Decimal) -> Position {
        Position::new(
            PositionId::new(id),
            MarketId::new(market_id),
            vec![PositionLeg::new(
                TokenId::new("token-1"),
                dec!(100),
                entry_cost / dec!(100),
            )],
            entry_cost,
            dec!(100), // guaranteed payout (volume)
            Utc::now(),
            PositionStatus::Open,
        )
    }

    #[test]
    fn close_reason_display() {
        assert_eq!(
            CloseReason::Settlement {
                winning_outcome: "Yes".to_string()
            }
            .to_string(),
            "settlement:Yes"
        );
        assert_eq!(CloseReason::ManualExit.to_string(), "manual_exit");
        assert_eq!(
            CloseReason::StopLoss {
                trigger_price: dec!(0.45)
            }
            .to_string(),
            "stop_loss:0.45"
        );
    }

    #[test]
    fn calculate_arbitrage_pnl_positive() {
        let position = make_position(1, "market-1", dec!(95));
        let pnl = PositionManager::calculate_arbitrage_pnl(&position, dec!(1.00));
        // 100 shares × $1.00 payout - $95 cost = $5 profit
        assert_eq!(pnl, dec!(5));
    }

    #[test]
    fn calculate_arbitrage_pnl_break_even() {
        let position = make_position(1, "market-1", dec!(100));
        let pnl = PositionManager::calculate_arbitrage_pnl(&position, dec!(1.00));
        assert_eq!(pnl, dec!(0));
    }
}
