//! Aggregation functions for statistics data.

use std::collections::HashMap;

use crate::port::inbound::operator::stats::StrategyStatsRecord;

/// Aggregate strategy breakdown rows by strategy name.
///
/// Combines multiple daily records for the same strategy into a single
/// aggregated record.
pub fn aggregate_by_strategy(rows: &[StrategyStatsRecord]) -> HashMap<String, StrategyStatsRecord> {
    let mut by_strategy: HashMap<String, StrategyStatsRecord> = HashMap::new();

    for row in rows {
        let entry =
            by_strategy
                .entry(row.strategy.clone())
                .or_insert_with(|| StrategyStatsRecord {
                    strategy: row.strategy.clone(),
                    ..Default::default()
                });
        entry.opportunities_detected += row.opportunities_detected;
        entry.opportunities_executed += row.opportunities_executed;
        entry.trades_opened += row.trades_opened;
        entry.trades_closed += row.trades_closed;
        entry.profit_realized += row.profit_realized;
        entry.win_count += row.win_count;
        entry.loss_count += row.loss_count;
    }

    by_strategy
}

/// Compute a percentage, returning None if the denominator is zero.
pub fn compute_percentage(numerator: i32, denominator: i32) -> Option<f64> {
    if denominator > 0 {
        Some(f64::from(numerator) / f64::from(denominator) * 100.0)
    } else {
        None
    }
}

/// Compute win rate percentage from win/loss counts.
pub fn compute_win_rate(win_count: i32, loss_count: i32) -> Option<f64> {
    let total = win_count + loss_count;
    compute_percentage(win_count, total)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for compute_percentage

    #[test]
    fn test_compute_percentage_normal_case() {
        let result = compute_percentage(50, 100);
        assert_eq!(result, Some(50.0));
    }

    #[test]
    fn test_compute_percentage_full_hundred() {
        let result = compute_percentage(100, 100);
        assert_eq!(result, Some(100.0));
    }

    #[test]
    fn test_compute_percentage_zero_numerator() {
        let result = compute_percentage(0, 100);
        assert_eq!(result, Some(0.0));
    }

    #[test]
    fn test_compute_percentage_zero_denominator_returns_none() {
        let result = compute_percentage(50, 0);
        assert!(result.is_none());
    }

    #[test]
    fn test_compute_percentage_negative_denominator_returns_none() {
        let result = compute_percentage(50, -10);
        assert!(result.is_none());
    }

    #[test]
    fn test_compute_percentage_fractional_result() {
        let result = compute_percentage(1, 3);
        assert!(result.is_some());
        let value = result.unwrap();
        assert!((value - 33.333_333_333_333_336).abs() < 0.0001);
    }

    // Tests for compute_win_rate

    #[test]
    fn test_compute_win_rate_all_wins() {
        let result = compute_win_rate(10, 0);
        assert_eq!(result, Some(100.0));
    }

    #[test]
    fn test_compute_win_rate_all_losses() {
        let result = compute_win_rate(0, 10);
        assert_eq!(result, Some(0.0));
    }

    #[test]
    fn test_compute_win_rate_fifty_percent() {
        let result = compute_win_rate(5, 5);
        assert_eq!(result, Some(50.0));
    }

    #[test]
    fn test_compute_win_rate_no_trades_returns_none() {
        let result = compute_win_rate(0, 0);
        assert!(result.is_none());
    }

    #[test]
    fn test_compute_win_rate_eighty_percent() {
        let result = compute_win_rate(8, 2);
        assert_eq!(result, Some(80.0));
    }

    // Tests for aggregate_by_strategy

    #[test]
    fn test_aggregate_by_strategy_empty_input() {
        let rows: Vec<StrategyStatsRecord> = vec![];
        let result = aggregate_by_strategy(&rows);
        assert!(result.is_empty());
    }

    #[test]
    fn test_aggregate_by_strategy_single_row() {
        let rows = vec![StrategyStatsRecord {
            strategy: "binary".to_string(),
            opportunities_detected: 10,
            opportunities_executed: 5,
            trades_opened: 5,
            trades_closed: 3,
            profit_realized: 100.0,
            win_count: 2,
            loss_count: 1,
        }];

        let result = aggregate_by_strategy(&rows);
        assert_eq!(result.len(), 1);
        assert!(result.contains_key("binary"));

        let binary = result.get("binary").unwrap();
        assert_eq!(binary.opportunities_detected, 10);
        assert_eq!(binary.opportunities_executed, 5);
        assert_eq!(binary.trades_opened, 5);
        assert_eq!(binary.trades_closed, 3);
        assert_eq!(binary.profit_realized, 100.0);
        assert_eq!(binary.win_count, 2);
        assert_eq!(binary.loss_count, 1);
    }

    #[test]
    fn test_aggregate_by_strategy_multiple_rows_same_strategy() {
        let rows = vec![
            StrategyStatsRecord {
                strategy: "binary".to_string(),
                opportunities_detected: 10,
                opportunities_executed: 5,
                trades_opened: 5,
                trades_closed: 3,
                profit_realized: 100.0,
                win_count: 2,
                loss_count: 1,
            },
            StrategyStatsRecord {
                strategy: "binary".to_string(),
                opportunities_detected: 20,
                opportunities_executed: 10,
                trades_opened: 10,
                trades_closed: 7,
                profit_realized: 200.0,
                win_count: 5,
                loss_count: 2,
            },
        ];

        let result = aggregate_by_strategy(&rows);
        assert_eq!(result.len(), 1);

        let binary = result.get("binary").unwrap();
        assert_eq!(binary.opportunities_detected, 30);
        assert_eq!(binary.opportunities_executed, 15);
        assert_eq!(binary.trades_opened, 15);
        assert_eq!(binary.trades_closed, 10);
        assert_eq!(binary.profit_realized, 300.0);
        assert_eq!(binary.win_count, 7);
        assert_eq!(binary.loss_count, 3);
    }

    #[test]
    fn test_aggregate_by_strategy_multiple_strategies() {
        let rows = vec![
            StrategyStatsRecord {
                strategy: "binary".to_string(),
                opportunities_detected: 10,
                opportunities_executed: 5,
                trades_opened: 5,
                trades_closed: 3,
                profit_realized: 100.0,
                win_count: 2,
                loss_count: 1,
            },
            StrategyStatsRecord {
                strategy: "multi".to_string(),
                opportunities_detected: 20,
                opportunities_executed: 10,
                trades_opened: 10,
                trades_closed: 7,
                profit_realized: 200.0,
                win_count: 5,
                loss_count: 2,
            },
        ];

        let result = aggregate_by_strategy(&rows);
        assert_eq!(result.len(), 2);
        assert!(result.contains_key("binary"));
        assert!(result.contains_key("multi"));

        let binary = result.get("binary").unwrap();
        assert_eq!(binary.opportunities_detected, 10);

        let multi = result.get("multi").unwrap();
        assert_eq!(multi.opportunities_detected, 20);
    }

    #[test]
    fn test_aggregate_by_strategy_preserves_strategy_name() {
        let rows = vec![StrategyStatsRecord {
            strategy: "test_strategy".to_string(),
            ..Default::default()
        }];

        let result = aggregate_by_strategy(&rows);
        let entry = result.get("test_strategy").unwrap();
        assert_eq!(entry.strategy, "test_strategy");
    }

    #[test]
    fn test_aggregate_by_strategy_handles_zero_values() {
        let rows = vec![
            StrategyStatsRecord {
                strategy: "zero".to_string(),
                opportunities_detected: 0,
                opportunities_executed: 0,
                trades_opened: 0,
                trades_closed: 0,
                profit_realized: 0.0,
                win_count: 0,
                loss_count: 0,
            },
            StrategyStatsRecord {
                strategy: "zero".to_string(),
                opportunities_detected: 0,
                opportunities_executed: 0,
                trades_opened: 0,
                trades_closed: 0,
                profit_realized: 0.0,
                win_count: 0,
                loss_count: 0,
            },
        ];

        let result = aggregate_by_strategy(&rows);
        let entry = result.get("zero").unwrap();
        assert_eq!(entry.opportunities_detected, 0);
        assert_eq!(entry.profit_realized, 0.0);
    }

    #[test]
    fn test_aggregate_by_strategy_negative_profit() {
        let rows = vec![
            StrategyStatsRecord {
                strategy: "loser".to_string(),
                profit_realized: -50.0,
                ..Default::default()
            },
            StrategyStatsRecord {
                strategy: "loser".to_string(),
                profit_realized: -30.0,
                ..Default::default()
            },
        ];

        let result = aggregate_by_strategy(&rows);
        let entry = result.get("loser").unwrap();
        assert_eq!(entry.profit_realized, -80.0);
    }
}
