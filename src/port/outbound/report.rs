//! Read-side reporting and query ports.
//!
//! Defines traits for querying trading statistics and status from persistent
//! storage. These are read-side contracts for the CQRS pattern.
//!
//! # Overview
//!
//! - [`StatusReportReader`]: Load current runtime status
//! - [`StatisticsReportReader`]: Query historical statistics

use chrono::NaiveDate;

use crate::domain::stats::StatsSummary;
use crate::error::Result;

/// Daily aggregate statistics record from storage.
#[derive(Debug, Clone, Default)]
pub struct DailyStatsRecord {
    /// Date in ISO 8601 format (YYYY-MM-DD).
    pub date: String,

    /// Total opportunities detected.
    pub opportunities_detected: i32,

    /// Total opportunities executed.
    pub opportunities_executed: i32,

    /// Total opportunities rejected by risk checks.
    pub opportunities_rejected: i32,

    /// Total trades opened.
    pub trades_opened: i32,

    /// Total trades closed.
    pub trades_closed: i32,

    /// Total realized profit in USD.
    pub profit_realized: f32,

    /// Total realized loss in USD.
    pub loss_realized: f32,

    /// Number of winning trades.
    pub win_count: i32,

    /// Number of losing trades.
    pub loss_count: i32,

    /// Total trading volume in USD.
    pub total_volume: f32,

    /// Peak exposure reached during the day in USD.
    pub peak_exposure: f32,

    /// Sum of latency measurements in milliseconds.
    pub latency_sum_ms: i32,

    /// Number of latency measurements.
    pub latency_count: i32,
}

/// Strategy-level daily statistics record from storage.
#[derive(Debug, Clone, Default)]
pub struct StrategyDailyStatsRecord {
    /// Date in ISO 8601 format (YYYY-MM-DD).
    pub date: String,

    /// Strategy name.
    pub strategy: String,

    /// Opportunities detected by this strategy.
    pub opportunities_detected: i32,

    /// Opportunities executed by this strategy.
    pub opportunities_executed: i32,

    /// Trades opened by this strategy.
    pub trades_opened: i32,

    /// Trades closed by this strategy.
    pub trades_closed: i32,

    /// Realized profit from this strategy in USD.
    pub profit_realized: f32,

    /// Winning trade count.
    pub win_count: i32,

    /// Losing trade count.
    pub loss_count: i32,
}

/// Recent activity item for status displays.
#[derive(Debug, Clone)]
pub enum RecentActivity {
    /// A trade was executed.
    Executed {
        /// Timestamp in human-readable format.
        timestamp: String,

        /// Realized profit in USD.
        profit: f32,

        /// Market description.
        market_description: String,
    },

    /// A trade was rejected.
    Rejected {
        /// Timestamp in human-readable format.
        timestamp: String,

        /// Rejection reason.
        reason: String,
    },
}

/// Status snapshot from persistent storage.
#[derive(Debug, Clone, Default)]
pub struct StatusSnapshot {
    /// Statistics for the current day.
    pub today: Option<DailyStatsRecord>,

    /// Daily statistics for the past week.
    pub week_rows: Vec<DailyStatsRecord>,

    /// Number of currently open positions.
    pub open_positions: i64,

    /// Number of distinct markets with positions.
    pub distinct_markets: i64,

    /// Current total exposure in USD.
    pub current_exposure: f32,

    /// Recent activity items.
    pub recent_activity: Vec<RecentActivity>,
}

/// Read-side port for loading current status.
///
/// # Thread Safety
///
/// Implementations must be thread-safe (`Send + Sync`).
pub trait StatusReportReader: Send + Sync {
    /// Load the current runtime status from storage.
    ///
    /// # Errors
    ///
    /// Returns an error if storage cannot be accessed.
    fn load_status(&self) -> Result<StatusSnapshot>;
}

/// Read-side port for querying historical statistics.
///
/// # Thread Safety
///
/// Implementations must be thread-safe (`Send + Sync`).
pub trait StatisticsReportReader: Send + Sync {
    /// Load aggregate statistics for a date range.
    ///
    /// # Arguments
    ///
    /// * `from` - Start date (inclusive).
    /// * `to` - End date (inclusive).
    ///
    /// # Errors
    ///
    /// Returns an error if storage cannot be accessed.
    fn load_summary(&self, from: NaiveDate, to: NaiveDate) -> Result<StatsSummary>;

    /// Load strategy-level daily records for a date range.
    ///
    /// # Arguments
    ///
    /// * `from` - Start date (inclusive).
    /// * `to` - End date (inclusive).
    ///
    /// # Errors
    ///
    /// Returns an error if storage cannot be accessed.
    fn load_strategy_breakdown(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<StrategyDailyStatsRecord>>;

    /// Load the count of currently open positions.
    ///
    /// # Errors
    ///
    /// Returns an error if storage cannot be accessed.
    fn load_open_positions(&self) -> Result<i64>;

    /// Load daily aggregate records for a date range.
    ///
    /// # Arguments
    ///
    /// * `from` - Start date (inclusive).
    /// * `to` - End date (inclusive).
    ///
    /// # Errors
    ///
    /// Returns an error if storage cannot be accessed.
    fn load_daily_rows(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<DailyStatsRecord>>;

    /// Export daily statistics as CSV.
    ///
    /// # Arguments
    ///
    /// * `from` - Start date (inclusive).
    /// * `to` - End date (inclusive).
    ///
    /// # Errors
    ///
    /// Returns an error if storage cannot be accessed.
    fn export_daily_csv(&self, from: NaiveDate, to: NaiveDate) -> Result<String>;

    /// Delete historical records older than the retention period.
    ///
    /// # Arguments
    ///
    /// * `retention_days` - Number of days of history to retain.
    ///
    /// # Errors
    ///
    /// Returns an error if storage cannot be accessed.
    fn prune_old_records(&self, retention_days: u32) -> Result<()>;
}
