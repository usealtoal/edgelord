//! Statistics recording port.
//!
//! Defines the write-side interface for persisting trading statistics.
//! This is the command side of the CQRS pattern for statistics.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::domain::{
    stats::RecordedOpportunity, stats::StatsSummary, stats::TradeCloseEvent, stats::TradeOpenEvent,
};

/// Write-side port for recording trading statistics.
///
/// Implementations persist opportunities, trades, and performance metrics
/// for historical analysis and reporting.
///
/// # Thread Safety
///
/// Implementations must be thread-safe (`Send + Sync`) as multiple components
/// may record statistics concurrently.
pub trait StatsRecorder: Send + Sync {
    /// Record an opportunity detection.
    ///
    /// # Arguments
    ///
    /// * `event` - Opportunity detection event.
    ///
    /// Returns the assigned opportunity ID if recording succeeds, or `None`
    /// if recording fails.
    fn record_opportunity(&self, event: &RecordedOpportunity) -> Option<i32>;

    /// Record a trade opening.
    ///
    /// # Arguments
    ///
    /// * `event` - Trade open event with position details.
    ///
    /// Returns the assigned trade ID if recording succeeds, or `None`
    /// if recording fails.
    fn record_trade_open(&self, event: &TradeOpenEvent) -> Option<i32>;

    /// Record a trade closing.
    ///
    /// # Arguments
    ///
    /// * `event` - Trade close event with final P&L.
    fn record_trade_close(&self, event: &TradeCloseEvent);

    /// Record a latency measurement.
    ///
    /// # Arguments
    ///
    /// * `latency_ms` - Measured latency in milliseconds.
    fn record_latency(&self, latency_ms: u32);

    /// Update the peak exposure metric if the current value exceeds the record.
    ///
    /// # Arguments
    ///
    /// * `exposure` - Current exposure amount.
    fn update_peak_exposure(&self, exposure: Decimal);

    /// Retrieve summary statistics for a date range.
    ///
    /// # Arguments
    ///
    /// * `from` - Start date (inclusive).
    /// * `to` - End date (inclusive).
    fn get_summary(&self, from: NaiveDate, to: NaiveDate) -> StatsSummary;

    /// Retrieve summary statistics for the current day.
    fn get_today(&self) -> StatsSummary;
}
