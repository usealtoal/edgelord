//! JSON serialization for statistics output.

use serde_json::{json, Value};

use crate::domain::stats::StatsSummary;
use crate::port::inbound::operator::statistics::{DailyStatsRecord, StrategyStatsRecord};

use super::aggregate::compute_win_rate;

/// Convert a StatsSummary to JSON.
pub fn summary_to_json(summary: &StatsSummary) -> Value {
    json!({
        "opportunities_detected": summary.opportunities_detected,
        "opportunities_executed": summary.opportunities_executed,
        "opportunities_rejected": summary.opportunities_rejected,
        "trades_opened": summary.trades_opened,
        "trades_closed": summary.trades_closed,
        "win_rate_pct": summary.win_rate(),
        "profit_realized": summary.profit_realized,
        "loss_realized": summary.loss_realized,
        "net_profit": summary.net_profit(),
        "total_volume": summary.total_volume,
    })
}

/// Convert strategy breakdown rows to JSON.
pub fn strategy_rows_to_json(rows: &[StrategyStatsRecord]) -> Value {
    let payload: Vec<_> = rows
        .iter()
        .map(|row| {
            let win_rate = compute_win_rate(row.win_count, row.loss_count);

            json!({
                "strategy": row.strategy,
                "opportunities_detected": row.opportunities_detected,
                "opportunities_executed": row.opportunities_executed,
                "trades_opened": row.trades_opened,
                "trades_closed": row.trades_closed,
                "profit_realized": row.profit_realized,
                "win_count": row.win_count,
                "loss_count": row.loss_count,
                "win_rate_pct": win_rate,
            })
        })
        .collect();
    json!(payload)
}

/// Convert daily breakdown rows to JSON.
pub fn daily_rows_to_json(rows: &[DailyStatsRecord]) -> Value {
    let payload: Vec<_> = rows
        .iter()
        .map(|row| {
            let win_rate = compute_win_rate(row.win_count, row.loss_count);

            json!({
                "date": row.date.to_string(),
                "opportunities_detected": row.opportunities_detected,
                "opportunities_executed": row.opportunities_executed,
                "opportunities_rejected": row.opportunities_rejected,
                "trades_opened": row.trades_opened,
                "trades_closed": row.trades_closed,
                "profit_realized": row.profit_realized,
                "loss_realized": row.loss_realized,
                "net_profit": row.profit_realized - row.loss_realized,
                "win_count": row.win_count,
                "loss_count": row.loss_count,
                "win_rate_pct": win_rate,
            })
        })
        .collect();
    json!(payload)
}
