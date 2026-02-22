//! Statistics projections for operator-facing adapters.

use chrono::NaiveDate;

use crate::domain::stats::StatsSummary;
use crate::error::Result;

/// Per-strategy aggregate row used by statistics views.
#[derive(Debug, Clone, Default)]
pub struct StrategyStatsRecord {
    pub strategy: String,
    pub opportunities_detected: i32,
    pub opportunities_executed: i32,
    pub trades_opened: i32,
    pub trades_closed: i32,
    pub profit_realized: f32,
    pub win_count: i32,
    pub loss_count: i32,
}

/// Daily aggregate row used by historical statistics views.
#[derive(Debug, Clone, Default)]
pub struct DailyStatsRecord {
    pub date: String,
    pub opportunities_detected: i32,
    pub opportunities_executed: i32,
    pub opportunities_rejected: i32,
    pub trades_opened: i32,
    pub trades_closed: i32,
    pub profit_realized: f32,
    pub loss_realized: f32,
    pub win_count: i32,
    pub loss_count: i32,
    pub total_volume: f32,
}

/// Statistics use-cases for operator-facing adapters.
pub trait StatisticsOperator: Send + Sync {
    /// Load aggregate stats summary for date range.
    fn load_summary(
        &self,
        database_url: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<StatsSummary>;

    /// Load per-strategy aggregate rows for date range.
    fn load_strategy_breakdown(
        &self,
        database_url: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<StrategyStatsRecord>>;

    /// Load open position count.
    fn load_open_positions(&self, database_url: &str) -> Result<i64>;

    /// Load daily aggregate rows for date range.
    fn load_daily_rows(
        &self,
        database_url: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<DailyStatsRecord>>;

    /// Export daily rows as CSV.
    fn export_daily_csv(
        &self,
        database_url: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<String>;

    /// Prune historical detailed records.
    fn prune_old_records(&self, database_url: &str, retention_days: u32) -> Result<()>;
}
