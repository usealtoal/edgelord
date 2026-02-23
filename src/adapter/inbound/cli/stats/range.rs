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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_today_range_has_same_start_and_end() {
        let range = DateRange::today();
        assert_eq!(range.start, range.end);
        assert_eq!(range.label, "Today");
    }

    #[test]
    fn test_today_range_is_current_date() {
        let range = DateRange::today();
        let expected = Utc::now().date_naive();
        assert_eq!(range.start, expected);
        assert_eq!(range.end, expected);
    }

    #[test]
    fn test_week_range_spans_seven_days() {
        let range = DateRange::week();
        let days_diff = (range.end - range.start).num_days();
        assert_eq!(days_diff, 7);
        assert_eq!(range.label, "Last 7 Days");
    }

    #[test]
    fn test_week_range_ends_today() {
        let range = DateRange::week();
        let today = Utc::now().date_naive();
        assert_eq!(range.end, today);
    }

    #[test]
    fn test_history_range_with_30_days() {
        let range = DateRange::history(30);
        let days_diff = (range.end - range.start).num_days();
        assert_eq!(days_diff, 30);
        assert_eq!(range.label, "Last 30 Days");
    }

    #[test]
    fn test_history_range_with_zero_days() {
        let range = DateRange::history(0);
        assert_eq!(range.start, range.end);
        assert_eq!(range.label, "Last 0 Days");
    }

    #[test]
    fn test_history_range_with_large_value() {
        let range = DateRange::history(365);
        let days_diff = (range.end - range.start).num_days();
        assert_eq!(days_diff, 365);
        assert_eq!(range.label, "Last 365 Days");
    }

    #[test]
    fn test_history_range_ends_today() {
        let range = DateRange::history(10);
        let today = Utc::now().date_naive();
        assert_eq!(range.end, today);
    }

    #[test]
    fn test_date_range_clone() {
        let original = DateRange::today();
        let cloned = original.clone();
        assert_eq!(original.start, cloned.start);
        assert_eq!(original.end, cloned.end);
        assert_eq!(original.label, cloned.label);
    }

    #[test]
    fn test_date_range_debug() {
        let range = DateRange::today();
        let debug_str = format!("{:?}", range);
        assert!(debug_str.contains("DateRange"));
        assert!(debug_str.contains("start"));
        assert!(debug_str.contains("end"));
        assert!(debug_str.contains("label"));
    }
}
