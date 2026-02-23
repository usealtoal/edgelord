//! Statistics domain types for trading performance tracking.
//!
//! This module provides DTOs (Data Transfer Objects) for recording and
//! analyzing trading statistics, including opportunities detected,
//! trades executed, and performance summaries.
//!
//! # Statistics Workflow
//!
//! 1. Opportunities are detected and recorded via [`RecordedOpportunity`]
//! 2. Executed trades are tracked with [`TradeOpenEvent`] and [`TradeCloseEvent`]
//! 3. Performance is summarized with [`StatsSummary`]
//!
//! # Examples
//!
//! Calculating performance metrics:
//!
//! ```
//! use edgelord::domain::stats::StatsSummary;
//! use rust_decimal_macros::dec;
//!
//! let mut summary = StatsSummary::default();
//! summary.win_count = 8;
//! summary.loss_count = 2;
//! summary.profit_realized = dec!(100.00);
//! summary.loss_realized = dec!(20.00);
//!
//! assert_eq!(summary.win_rate(), Some(80.0));
//! assert_eq!(summary.net_profit(), dec!(80.00));
//! ```

use rust_decimal::Decimal;

/// A recorded opportunity for statistics tracking.
///
/// Captures details about a detected arbitrage opportunity, whether
/// it was executed, and if rejected, why.
#[derive(Debug, Clone)]
pub struct RecordedOpportunity {
    /// Name of the strategy that detected this opportunity.
    pub strategy: String,
    /// Market IDs involved in this opportunity.
    pub market_ids: Vec<String>,
    /// Edge per share (payout minus cost).
    pub edge: Decimal,
    /// Expected profit if fully executed.
    pub expected_profit: Decimal,
    /// Whether this opportunity was executed.
    pub executed: bool,
    /// Reason for rejection if not executed.
    pub rejected_reason: Option<String>,
}

/// Event recorded when a trade is opened.
///
/// Links the trade to its originating opportunity for tracking purposes.
#[derive(Debug, Clone)]
pub struct TradeOpenEvent {
    /// ID of the opportunity that triggered this trade.
    pub opportunity_id: i32,
    /// Strategy name that opened the trade.
    pub strategy: String,
    /// Market IDs involved in this trade.
    pub market_ids: Vec<String>,
    /// Individual legs of the trade.
    pub legs: Vec<TradeLeg>,
    /// Number of shares traded.
    pub size: Decimal,
    /// Expected profit from this trade.
    pub expected_profit: Decimal,
}

/// A single leg of a trade for statistics recording.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TradeLeg {
    /// Token ID of this leg.
    pub token_id: String,
    /// Order side ("buy" or "sell").
    pub side: String,
    /// Execution price.
    pub price: Decimal,
    /// Size in shares.
    pub size: Decimal,
}

/// Event recorded when a trade is closed.
#[derive(Debug, Clone)]
pub struct TradeCloseEvent {
    /// ID of the trade being closed.
    pub trade_id: i32,
    /// Actual realized profit or loss.
    pub realized_profit: Decimal,
    /// Reason for closing (e.g., "market_settled", "manual_exit").
    pub reason: String,
}

/// Summary statistics for a time period.
///
/// Aggregates trading performance metrics for analysis and reporting.
#[derive(Debug, Clone, Default)]
pub struct StatsSummary {
    /// Total opportunities detected.
    pub opportunities_detected: i64,
    /// Opportunities that were executed.
    pub opportunities_executed: i64,
    /// Opportunities that were rejected.
    pub opportunities_rejected: i64,
    /// Total trades opened.
    pub trades_opened: i64,
    /// Total trades closed.
    pub trades_closed: i64,
    /// Total profit from winning trades.
    pub profit_realized: Decimal,
    /// Total loss from losing trades.
    pub loss_realized: Decimal,
    /// Number of winning trades.
    pub win_count: i64,
    /// Number of losing trades.
    pub loss_count: i64,
    /// Total volume traded.
    pub total_volume: Decimal,
}

impl StatsSummary {
    /// Calculates the win rate as a percentage.
    ///
    /// Returns `None` if no trades have been closed.
    #[must_use]
    pub fn win_rate(&self) -> Option<f64> {
        let total = self.win_count + self.loss_count;
        if total == 0 {
            None
        } else {
            Some(self.win_count as f64 / total as f64 * 100.0)
        }
    }

    /// Calculates the net profit (total profit minus total loss).
    #[must_use]
    pub fn net_profit(&self) -> Decimal {
        self.profit_realized - self.loss_realized
    }
}

/// Summary of an opportunity for display purposes.
#[derive(Debug, Clone)]
pub struct OpportunitySummary {
    /// Unique identifier for this opportunity.
    pub id: i32,
    /// Strategy that detected this opportunity.
    pub strategy: String,
    /// Edge per share.
    pub edge: Decimal,
    /// Expected profit if executed.
    pub expected_profit: Decimal,
    /// Whether this opportunity was executed.
    pub executed: bool,
    /// Reason for rejection if not executed.
    pub rejected_reason: Option<String>,
    /// Timestamp when detected (ISO 8601 format).
    pub detected_at: String,
}
