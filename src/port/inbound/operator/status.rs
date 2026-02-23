//! Status projection types for operator-facing adapters.
//!
//! Defines view models for current runtime status displays through
//! operator interfaces like the CLI and Telegram bot.

use crate::error::Result;

/// Recent activity item for status displays.
///
/// Represents a single recent event for display in activity feeds.
#[derive(Debug, Clone)]
pub enum RecentActivity {
    /// A trade was successfully executed.
    Executed {
        /// Timestamp in human-readable format.
        timestamp: String,

        /// Realized profit in USD.
        profit: f32,

        /// Description of the market traded.
        market_description: String,
    },

    /// A trade was rejected by risk checks.
    Rejected {
        /// Timestamp in human-readable format.
        timestamp: String,

        /// Rejection reason.
        reason: String,
    },
}

/// Summary of trading activity for the current day.
#[derive(Debug, Clone)]
pub struct DailyStatusSummary {
    /// Opportunities detected today.
    pub opportunities_detected: i32,

    /// Opportunities executed today.
    pub opportunities_executed: i32,

    /// Opportunities rejected by risk checks today.
    pub opportunities_rejected: i32,

    /// Realized profit today in USD.
    pub profit_realized: f32,

    /// Realized loss today in USD.
    pub loss_realized: f32,
}

/// Current runtime status snapshot.
///
/// Contains a point-in-time view of system status for display.
#[derive(Debug, Clone)]
pub struct StatusSnapshot {
    /// Summary for the current day, if available.
    pub today: Option<DailyStatusSummary>,

    /// Number of currently open positions.
    pub open_positions: i64,

    /// Number of distinct markets with positions.
    pub distinct_markets: i64,

    /// Current total exposure in USD.
    pub current_exposure: f32,

    /// Recent activity items for display.
    pub recent_activity: Vec<RecentActivity>,
}

/// Status use-cases for operator-facing adapters.
///
/// # Thread Safety
///
/// Implementations must be thread-safe (`Send + Sync`).
pub trait StatusOperator: Send + Sync {
    /// Return a display-ready network label.
    ///
    /// # Arguments
    ///
    /// * `config_toml` - Raw TOML configuration content.
    ///
    /// Returns a string like "mainnet (polygon)" for display.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration cannot be parsed.
    fn network_label(&self, config_toml: &str) -> Result<String>;

    /// Load the current status snapshot from the database.
    ///
    /// # Arguments
    ///
    /// * `database_url` - Path to the statistics database.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be accessed.
    fn load_status(&self, database_url: &str) -> Result<StatusSnapshot>;
}
