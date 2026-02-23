//! Status projections for operator-facing adapters.

use crate::error::Result;

/// A recent activity item for operator-facing status displays.
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

/// Daily status summary for "today" views.
#[derive(Debug, Clone)]
pub struct DailyStatusSummary {
    pub opportunities_detected: i32,
    pub opportunities_executed: i32,
    pub opportunities_rejected: i32,
    pub profit_realized: f32,
    pub loss_realized: f32,
}

/// Snapshot of current runtime status.
#[derive(Debug, Clone)]
pub struct StatusSnapshot {
    pub today: Option<DailyStatusSummary>,
    pub open_positions: i64,
    pub distinct_markets: i64,
    pub current_exposure: f32,
    pub recent_activity: Vec<RecentActivity>,
}

/// Status use-cases for operator-facing adapters.
pub trait StatusOperator: Send + Sync {
    /// Returns a display-ready network label (for example "mainnet (polygon)").
    fn network_label(&self, config_toml: &str) -> Result<String>;

    /// Load current status snapshot from database.
    fn load_status(&self, database_url: &str) -> Result<StatusSnapshot>;
}
