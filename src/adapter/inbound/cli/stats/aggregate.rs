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
        let entry = by_strategy
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
