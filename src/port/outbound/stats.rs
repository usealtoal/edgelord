//! Statistics recording port.
//!
//! Defines the interface for recording trading statistics.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::domain::{
    stats::RecordedOpportunity, stats::StatsSummary, stats::TradeCloseEvent, stats::TradeOpenEvent,
};

/// Port for recording trading statistics.
///
/// Implementations persist opportunities, trades, and performance metrics
/// for historical analysis and reporting.
pub trait StatsRecorder: Send + Sync {
    /// Record an opportunity detection.
    ///
    /// Returns the opportunity ID if successfully recorded.
    fn record_opportunity(&self, event: &RecordedOpportunity) -> Option<i32>;

    /// Record a trade opening.
    ///
    /// Returns the trade ID if successfully recorded.
    fn record_trade_open(&self, event: &TradeOpenEvent) -> Option<i32>;

    /// Record a trade closing.
    fn record_trade_close(&self, event: &TradeCloseEvent);

    /// Record a latency sample in milliseconds.
    fn record_latency(&self, latency_ms: u32);

    /// Update peak exposure if current value is higher.
    fn update_peak_exposure(&self, exposure: Decimal);

    /// Get summary statistics for a date range.
    fn get_summary(&self, from: NaiveDate, to: NaiveDate) -> StatsSummary;

    /// Get today's summary statistics.
    fn get_today(&self) -> StatsSummary;
}
