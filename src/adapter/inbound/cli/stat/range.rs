//! Date range calculation for statistics queries.

use chrono::{Duration, NaiveDate, Utc};

/// A date range for statistics queries.
#[derive(Debug, Clone)]
pub struct DateRange {
    /// Start date (inclusive).
    pub start: NaiveDate,
    /// End date (inclusive).
    pub end: NaiveDate,
    /// Human-readable label for the range.
    pub label: String,
}

impl DateRange {
    /// Create a range for today only.
    pub fn today() -> Self {
        let today = Utc::now().date_naive();
        Self {
            start: today,
            end: today,
            label: "Today".to_string(),
        }
    }

    /// Create a range for the last 7 days.
    pub fn week() -> Self {
        let today = Utc::now().date_naive();
        let week_ago = today - Duration::days(7);
        Self {
            start: week_ago,
            end: today,
            label: "Last 7 Days".to_string(),
        }
    }

    /// Create a range for the last N days.
    pub fn history(days: u32) -> Self {
        let today = Utc::now().date_naive();
        let start = today - Duration::days(i64::from(days));
        Self {
            start,
            end: today,
            label: format!("Last {days} Days"),
        }
    }
}
