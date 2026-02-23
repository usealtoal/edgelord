//! JSON serialization for statistics output.

use serde_json::{json, Value};

use crate::domain::stats::StatsSummary;
use crate::port::inbound::operator::stats::{DailyStatsRecord, StrategyStatsRecord};

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

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // Tests for summary_to_json

    #[test]
    fn test_summary_to_json_default_values() {
        let summary = StatsSummary::default();
        let json = summary_to_json(&summary);

        assert_eq!(json["opportunities_detected"], 0);
        assert_eq!(json["opportunities_executed"], 0);
        assert_eq!(json["opportunities_rejected"], 0);
        assert_eq!(json["trades_opened"], 0);
        assert_eq!(json["trades_closed"], 0);
        assert!(json["win_rate_pct"].is_null());
        assert_eq!(json["profit_realized"], "0");
        assert_eq!(json["loss_realized"], "0");
        assert_eq!(json["net_profit"], "0");
        assert_eq!(json["total_volume"], "0");
    }

    #[test]
    fn test_summary_to_json_with_values() {
        let summary = StatsSummary {
            opportunities_detected: 100,
            opportunities_executed: 80,
            opportunities_rejected: 20,
            trades_opened: 50,
            trades_closed: 45,
            profit_realized: dec!(1000.50),
            loss_realized: dec!(200.25),
            win_count: 35,
            loss_count: 10,
            total_volume: dec!(50000.00),
        };
        let json = summary_to_json(&summary);

        assert_eq!(json["opportunities_detected"], 100);
        assert_eq!(json["opportunities_executed"], 80);
        assert_eq!(json["opportunities_rejected"], 20);
        assert_eq!(json["trades_opened"], 50);
        assert_eq!(json["trades_closed"], 45);
        // Win rate: 35 / (35 + 10) = 77.78%
        let win_rate = json["win_rate_pct"].as_f64().unwrap();
        assert!((win_rate - 77.77777777777777).abs() < 0.001);
    }

    #[test]
    fn test_summary_to_json_win_rate_null_when_no_trades() {
        let summary = StatsSummary {
            win_count: 0,
            loss_count: 0,
            ..Default::default()
        };
        let json = summary_to_json(&summary);
        assert!(json["win_rate_pct"].is_null());
    }

    #[test]
    fn test_summary_to_json_net_profit_calculation() {
        let summary = StatsSummary {
            profit_realized: dec!(500.00),
            loss_realized: dec!(200.00),
            ..Default::default()
        };
        let json = summary_to_json(&summary);
        // Decimal serialization preserves precision
        assert_eq!(json["net_profit"], "300.00");
    }

    // Tests for strategy_rows_to_json

    #[test]
    fn test_strategy_rows_to_json_empty() {
        let rows: Vec<StrategyStatsRecord> = vec![];
        let json = strategy_rows_to_json(&rows);
        assert!(json.as_array().unwrap().is_empty());
    }

    #[test]
    fn test_strategy_rows_to_json_single_row() {
        let rows = vec![StrategyStatsRecord {
            strategy: "binary".to_string(),
            opportunities_detected: 10,
            opportunities_executed: 8,
            trades_opened: 8,
            trades_closed: 6,
            profit_realized: 150.0,
            win_count: 5,
            loss_count: 1,
        }];

        let json = strategy_rows_to_json(&rows);
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 1);

        let row = &arr[0];
        assert_eq!(row["strategy"], "binary");
        assert_eq!(row["opportunities_detected"], 10);
        assert_eq!(row["opportunities_executed"], 8);
        assert_eq!(row["trades_opened"], 8);
        assert_eq!(row["trades_closed"], 6);
        assert_eq!(row["profit_realized"], 150.0);
        assert_eq!(row["win_count"], 5);
        assert_eq!(row["loss_count"], 1);

        // Win rate: 5 / (5 + 1) = 83.33%
        let win_rate = row["win_rate_pct"].as_f64().unwrap();
        assert!((win_rate - 83.33333333333333).abs() < 0.001);
    }

    #[test]
    fn test_strategy_rows_to_json_multiple_rows() {
        let rows = vec![
            StrategyStatsRecord {
                strategy: "binary".to_string(),
                ..Default::default()
            },
            StrategyStatsRecord {
                strategy: "multi".to_string(),
                ..Default::default()
            },
        ];

        let json = strategy_rows_to_json(&rows);
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["strategy"], "binary");
        assert_eq!(arr[1]["strategy"], "multi");
    }

    #[test]
    fn test_strategy_rows_to_json_win_rate_null_when_no_trades() {
        let rows = vec![StrategyStatsRecord {
            strategy: "untested".to_string(),
            win_count: 0,
            loss_count: 0,
            ..Default::default()
        }];

        let json = strategy_rows_to_json(&rows);
        let arr = json.as_array().unwrap();
        assert!(arr[0]["win_rate_pct"].is_null());
    }

    // Tests for daily_rows_to_json

    #[test]
    fn test_daily_rows_to_json_empty() {
        let rows: Vec<DailyStatsRecord> = vec![];
        let json = daily_rows_to_json(&rows);
        assert!(json.as_array().unwrap().is_empty());
    }

    #[test]
    fn test_daily_rows_to_json_single_row() {
        let rows = vec![DailyStatsRecord {
            date: "2024-01-15".to_string(),
            opportunities_detected: 50,
            opportunities_executed: 40,
            opportunities_rejected: 10,
            trades_opened: 25,
            trades_closed: 20,
            profit_realized: 500.0,
            loss_realized: 100.0,
            win_count: 15,
            loss_count: 5,
            total_volume: 10000.0,
        }];

        let json = daily_rows_to_json(&rows);
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 1);

        let row = &arr[0];
        assert_eq!(row["date"], "2024-01-15");
        assert_eq!(row["opportunities_detected"], 50);
        assert_eq!(row["opportunities_executed"], 40);
        assert_eq!(row["opportunities_rejected"], 10);
        assert_eq!(row["trades_opened"], 25);
        assert_eq!(row["trades_closed"], 20);
        assert_eq!(row["profit_realized"], 500.0);
        assert_eq!(row["loss_realized"], 100.0);
        assert_eq!(row["net_profit"], 400.0);
        assert_eq!(row["win_count"], 15);
        assert_eq!(row["loss_count"], 5);
    }

    #[test]
    fn test_daily_rows_to_json_net_profit_calculation() {
        let rows = vec![DailyStatsRecord {
            date: "2024-01-15".to_string(),
            profit_realized: 300.0,
            loss_realized: 150.0,
            ..Default::default()
        }];

        let json = daily_rows_to_json(&rows);
        let row = &json.as_array().unwrap()[0];
        assert_eq!(row["net_profit"], 150.0);
    }

    #[test]
    fn test_daily_rows_to_json_negative_net_profit() {
        let rows = vec![DailyStatsRecord {
            date: "2024-01-15".to_string(),
            profit_realized: 100.0,
            loss_realized: 250.0,
            ..Default::default()
        }];

        let json = daily_rows_to_json(&rows);
        let row = &json.as_array().unwrap()[0];
        assert_eq!(row["net_profit"], -150.0);
    }

    #[test]
    fn test_daily_rows_to_json_multiple_rows() {
        let rows = vec![
            DailyStatsRecord {
                date: "2024-01-15".to_string(),
                ..Default::default()
            },
            DailyStatsRecord {
                date: "2024-01-16".to_string(),
                ..Default::default()
            },
            DailyStatsRecord {
                date: "2024-01-17".to_string(),
                ..Default::default()
            },
        ];

        let json = daily_rows_to_json(&rows);
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0]["date"], "2024-01-15");
        assert_eq!(arr[1]["date"], "2024-01-16");
        assert_eq!(arr[2]["date"], "2024-01-17");
    }

    #[test]
    fn test_daily_rows_to_json_win_rate_calculation() {
        let rows = vec![DailyStatsRecord {
            date: "2024-01-15".to_string(),
            win_count: 8,
            loss_count: 2,
            ..Default::default()
        }];

        let json = daily_rows_to_json(&rows);
        let row = &json.as_array().unwrap()[0];
        let win_rate = row["win_rate_pct"].as_f64().unwrap();
        assert_eq!(win_rate, 80.0);
    }
}
