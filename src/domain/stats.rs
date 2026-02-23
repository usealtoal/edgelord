//! Statistics domain types.
//!
//! DTOs for recording trading statistics.

use rust_decimal::Decimal;

/// Recorded opportunity for stats tracking.
#[derive(Debug, Clone)]
pub struct RecordedOpportunity {
    pub strategy: String,
    pub market_ids: Vec<String>,
    pub edge: Decimal,
    pub expected_profit: Decimal,
    pub executed: bool,
    pub rejected_reason: Option<String>,
}

/// Recorded trade open event.
#[derive(Debug, Clone)]
pub struct TradeOpenEvent {
    pub opportunity_id: i32,
    pub strategy: String,
    pub market_ids: Vec<String>,
    pub legs: Vec<TradeLeg>,
    pub size: Decimal,
    pub expected_profit: Decimal,
}

/// A single leg of a trade.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TradeLeg {
    pub token_id: String,
    pub side: String,
    pub price: Decimal,
    pub size: Decimal,
}

/// Recorded trade close event.
#[derive(Debug, Clone)]
pub struct TradeCloseEvent {
    pub trade_id: i32,
    pub realized_profit: Decimal,
    pub reason: String,
}

/// Summary statistics for a time period.
#[derive(Debug, Clone, Default)]
pub struct StatsSummary {
    pub opportunities_detected: i64,
    pub opportunities_executed: i64,
    pub opportunities_rejected: i64,
    pub trades_opened: i64,
    pub trades_closed: i64,
    pub profit_realized: Decimal,
    pub loss_realized: Decimal,
    pub win_count: i64,
    pub loss_count: i64,
    pub total_volume: Decimal,
}

impl StatsSummary {
    /// Calculate win rate as a percentage.
    #[must_use]
    pub fn win_rate(&self) -> Option<f64> {
        let total = self.win_count + self.loss_count;
        if total == 0 {
            None
        } else {
            Some(self.win_count as f64 / total as f64 * 100.0)
        }
    }

    /// Calculate net profit.
    #[must_use]
    pub fn net_profit(&self) -> Decimal {
        self.profit_realized - self.loss_realized
    }
}

/// Summary of an opportunity for display.
#[derive(Debug, Clone)]
pub struct OpportunitySummary {
    pub id: i32,
    pub strategy: String,
    pub edge: Decimal,
    pub expected_profit: Decimal,
    pub executed: bool,
    pub rejected_reason: Option<String>,
    pub detected_at: String,
}
