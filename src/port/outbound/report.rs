//! Read-side reporting/query ports.

use chrono::NaiveDate;

use crate::domain::stats::StatsSummary;
use crate::error::Result;

/// Daily aggregate record returned by report queries.
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
    pub peak_exposure: f32,
    pub latency_sum_ms: i32,
    pub latency_count: i32,
}

/// Strategy-level daily aggregate record returned by report queries.
#[derive(Debug, Clone, Default)]
pub struct StrategyDailyStatsRecord {
    pub date: String,
    pub strategy: String,
    pub opportunities_detected: i32,
    pub opportunities_executed: i32,
    pub trades_opened: i32,
    pub trades_closed: i32,
    pub profit_realized: f32,
    pub win_count: i32,
    pub loss_count: i32,
}

/// Recent activity item for status surfaces.
#[derive(Debug, Clone)]
pub enum RecentActivity {
    Executed {
        timestamp: String,
        profit: f32,
        market_description: String,
    },
    Rejected {
        timestamp: String,
        reason: String,
    },
}

/// Status snapshot returned by read-side queries.
#[derive(Debug, Clone, Default)]
pub struct StatusSnapshot {
    pub today: Option<DailyStatsRecord>,
    pub week_rows: Vec<DailyStatsRecord>,
    pub open_positions: i64,
    pub distinct_markets: i64,
    pub current_exposure: f32,
    pub recent_activity: Vec<RecentActivity>,
}

/// Read-side status query contract.
pub trait StatusReportReader: Send + Sync {
    /// Load current runtime status from persistent storage.
    fn load_status(&self) -> Result<StatusSnapshot>;
}

/// Read-side statistics query contract.
pub trait StatisticsReportReader: Send + Sync {
    /// Load aggregate summary for date range.
    fn load_summary(&self, from: NaiveDate, to: NaiveDate) -> Result<StatsSummary>;

    /// Load strategy-level daily rows for date range.
    fn load_strategy_breakdown(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<StrategyDailyStatsRecord>>;

    /// Load count of currently open positions.
    fn load_open_positions(&self) -> Result<i64>;

    /// Load daily rows for date range.
    fn load_daily_rows(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<DailyStatsRecord>>;

    /// Export daily rows as CSV.
    fn export_daily_csv(&self, from: NaiveDate, to: NaiveDate) -> Result<String>;

    /// Prune old detailed records.
    fn prune_old_records(&self, retention_days: u32) -> Result<()>;
}
