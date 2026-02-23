//! Statistics projection types for operator-facing adapters.
//!
//! Defines view models for trading statistics and performance reporting
//! through operator interfaces like the CLI.

use chrono::NaiveDate;

use crate::domain::stats::StatsSummary;
use crate::error::Result;

/// Per-strategy aggregate statistics record.
///
/// Contains summarized metrics for a single strategy over a date range.
#[derive(Debug, Clone, Default)]
pub struct StrategyStatsRecord {
    /// Strategy name.
    pub strategy: String,

    /// Total opportunities detected by this strategy.
    pub opportunities_detected: i32,

    /// Total opportunities executed by this strategy.
    pub opportunities_executed: i32,

    /// Total trades opened.
    pub trades_opened: i32,

    /// Total trades closed.
    pub trades_closed: i32,

    /// Total realized profit in USD.
    pub profit_realized: f32,

    /// Number of winning trades.
    pub win_count: i32,

    /// Number of losing trades.
    pub loss_count: i32,
}

/// Daily aggregate statistics record.
///
/// Contains summarized metrics for a single calendar day.
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
}

/// Statistics use-cases for operator-facing adapters.
///
/// # Thread Safety
///
/// Implementations must be thread-safe (`Send + Sync`).
pub trait StatisticsOperator: Send + Sync {
    /// Load aggregate statistics summary for a date range.
    ///
    /// # Arguments
    ///
    /// * `database_url` - Path to the statistics database.
    /// * `from` - Start date (inclusive).
    /// * `to` - End date (inclusive).
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be accessed.
    fn load_summary(
        &self,
        database_url: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<StatsSummary>;

    /// Load per-strategy breakdown for a date range.
    ///
    /// # Arguments
    ///
    /// * `database_url` - Path to the statistics database.
    /// * `from` - Start date (inclusive).
    /// * `to` - End date (inclusive).
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be accessed.
    fn load_strategy_breakdown(
        &self,
        database_url: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<StrategyStatsRecord>>;

    /// Load the count of currently open positions.
    ///
    /// # Arguments
    ///
    /// * `database_url` - Path to the statistics database.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be accessed.
    fn load_open_positions(&self, database_url: &str) -> Result<i64>;

    /// Load daily aggregate rows for a date range.
    ///
    /// # Arguments
    ///
    /// * `database_url` - Path to the statistics database.
    /// * `from` - Start date (inclusive).
    /// * `to` - End date (inclusive).
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be accessed.
    fn load_daily_rows(
        &self,
        database_url: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<DailyStatsRecord>>;

    /// Export daily statistics as CSV.
    ///
    /// # Arguments
    ///
    /// * `database_url` - Path to the statistics database.
    /// * `from` - Start date (inclusive).
    /// * `to` - End date (inclusive).
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be accessed.
    fn export_daily_csv(
        &self,
        database_url: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<String>;

    /// Delete historical records older than the retention period.
    ///
    /// # Arguments
    ///
    /// * `database_url` - Path to the statistics database.
    /// * `retention_days` - Number of days of history to retain.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be accessed.
    fn prune_old_records(&self, database_url: &str, retention_days: u32) -> Result<()>;
}
