//! Handler for the `statistics` command group.

use std::path::Path;

use chrono::{Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde_json::json;

use crate::adapter::inbound::cli::{operator, output};
use crate::domain::stats::StatsSummary;
use crate::error::Result;
use crate::port::inbound::operator::statistics::{
    DailyStatsRecord as DailyStatsRow, StrategyStatsRecord as StrategyDailyStatsRow,
};

fn load_summary(database_url: &str, from: NaiveDate, to: NaiveDate) -> Result<StatsSummary> {
    operator::operator().load_summary(database_url, from, to)
}

fn load_strategy_breakdown(
    database_url: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<StrategyDailyStatsRow>> {
    operator::operator().load_strategy_breakdown(database_url, from, to)
}

fn load_open_positions(database_url: &str) -> Result<i64> {
    operator::operator().load_open_positions(database_url)
}

fn date_range_today() -> (NaiveDate, NaiveDate, String) {
    let today = Utc::now().date_naive();
    (today, today, "Today".to_string())
}

fn date_range_week() -> (NaiveDate, NaiveDate, String) {
    let today = Utc::now().date_naive();
    let week_ago = today - Duration::days(7);
    (week_ago, today, "Last 7 Days".to_string())
}

fn date_range_history(days: u32) -> (NaiveDate, NaiveDate, String) {
    let today = Utc::now().date_naive();
    let start = today - Duration::days(i64::from(days));
    (start, today, format!("Last {days} Days"))
}

fn load_daily_rows(
    database_url: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<DailyStatsRow>> {
    operator::operator().load_daily_rows(database_url, from, to)
}

fn export_daily_csv(database_url: &str, from: NaiveDate, to: NaiveDate) -> Result<String> {
    operator::operator().export_daily_csv(database_url, from, to)
}

fn prune_old_records(database_url: &str, retention_days: u32) -> Result<()> {
    operator::operator().prune_old_records(database_url, retention_days)
}

/// Execute `statistics` (default: today).
pub fn execute_today(db_path: &Path) -> Result<()> {
    if output::is_quiet() && !output::is_json() {
        return Ok(());
    }

    let database_url = operator::sqlite_database_url(db_path);
    let (from, to, label) = date_range_today();
    let summary = load_summary(&database_url, from, to)?;
    let rows = load_strategy_breakdown(&database_url, from, to)?;
    let open_positions = load_open_positions(&database_url)?;

    if output::is_json() {
        println!(
            "{}",
            json!({
                "command": "statistics.today",
                "label": label,
                "summary": summary_to_json(&summary),
                "strategy_breakdown": strategy_rows_to_json(&rows),
                "open_positions": open_positions,
            })
        );
        return Ok(());
    }

    print_summary(&summary, &label)?;
    print_strategy_breakdown(&rows)?;
    print_open_positions(open_positions);

    Ok(())
}

/// Execute `statistics week`.
pub fn execute_week(db_path: &Path) -> Result<()> {
    if output::is_quiet() && !output::is_json() {
        return Ok(());
    }

    let database_url = operator::sqlite_database_url(db_path);
    let (from, to, label) = date_range_week();
    let summary = load_summary(&database_url, from, to)?;
    let rows = load_strategy_breakdown(&database_url, from, to)?;

    if output::is_json() {
        println!(
            "{}",
            json!({
                "command": "statistics.week",
                "label": label,
                "summary": summary_to_json(&summary),
                "strategy_breakdown": strategy_rows_to_json(&rows),
            })
        );
        return Ok(());
    }

    print_summary(&summary, &label)?;
    print_strategy_breakdown(&rows)?;

    Ok(())
}

/// Execute `statistics history [days]`.
pub fn execute_history(db_path: &Path, days: u32) -> Result<()> {
    if output::is_quiet() && !output::is_json() {
        return Ok(());
    }

    let database_url = operator::sqlite_database_url(db_path);
    let (from, to, label) = date_range_history(days);
    let summary = load_summary(&database_url, from, to)?;
    let rows = load_daily_rows(&database_url, from, to)?;

    if output::is_json() {
        println!(
            "{}",
            json!({
                "command": "statistics.history",
                "label": label,
                "days": days,
                "from": from.to_string(),
                "to": to.to_string(),
                "summary": summary_to_json(&summary),
                "daily_breakdown": daily_rows_to_json(&rows),
            })
        );
        return Ok(());
    }

    print_summary(&summary, &label)?;
    print_daily_breakdown(&rows)?;

    Ok(())
}

fn print_summary(summary: &StatsSummary, label: &str) -> Result<()> {
    output::section(label);
    output::section("Opportunities");
    output::field("Detected", summary.opportunities_detected);
    output::field(
        "Executed",
        format!(
            "{} ({:.1}%)",
            summary.opportunities_executed,
            if summary.opportunities_detected > 0 {
                summary.opportunities_executed as f64 / summary.opportunities_detected as f64
                    * 100.0
            } else {
                0.0
            }
        ),
    );
    output::field("Rejected", summary.opportunities_rejected);

    output::section("Trades");
    output::field("Opened", summary.trades_opened);
    output::field("Closed", summary.trades_closed);
    output::field(
        "Win rate",
        format!(
            "{}%",
            summary
                .win_rate()
                .map(|r| format!("{r:.1}"))
                .unwrap_or_else(|| "N/A".to_string())
        ),
    );

    output::section("Profit/Loss");
    output::field("Profit", format!("${:.2}", summary.profit_realized));
    output::field("Loss", format!("${:.2}", summary.loss_realized));
    output::field(
        "Net",
        format!(
            "${:>8.2} {}",
            summary.net_profit(),
            if summary.net_profit() >= Decimal::ZERO {
                "+"
            } else {
                "-"
            }
        ),
    );
    output::field("Volume", format!("${:.2}", summary.total_volume));

    Ok(())
}

fn print_strategy_breakdown(rows: &[StrategyDailyStatsRow]) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }

    let mut by_strategy: std::collections::HashMap<String, StrategyDailyStatsRow> =
        std::collections::HashMap::new();

    for row in rows {
        let entry =
            by_strategy
                .entry(row.strategy.clone())
                .or_insert_with(|| StrategyDailyStatsRow {
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

    output::section("By Strategy");
    println!(
        "  {:20} {:>8} {:>8} {:>10} {:>8}",
        "Strategy", "Opps", "Trades", "Profit", "Win %"
    );
    println!("  {:─<20} {:─>8} {:─>8} {:─>10} {:─>8}", "", "", "", "", "");

    for (name, stats_row) in &by_strategy {
        let total = stats_row.win_count + stats_row.loss_count;
        let win_rate = if total > 0 {
            format!("{:.1}%", stats_row.win_count as f64 / total as f64 * 100.0)
        } else {
            "N/A".to_string()
        };
        println!(
            "  {:20} {:>8} {:>8} ${:>9.2} {:>8}",
            name,
            stats_row.opportunities_detected,
            stats_row.trades_closed,
            stats_row.profit_realized,
            win_rate
        );
    }

    Ok(())
}

fn print_daily_breakdown(rows: &[DailyStatsRow]) -> Result<()> {
    if rows.is_empty() {
        println!("  No data for this period.");
        return Ok(());
    }

    output::section("Daily Breakdown");
    println!(
        "  {:12} {:>6} {:>6} {:>10} {:>8}",
        "Date", "Opps", "Trades", "Net P/L", "Win %"
    );
    println!("  {:─<12} {:─>6} {:─>6} {:─>10} {:─>8}", "", "", "", "", "");

    for row in rows {
        let total = row.win_count + row.loss_count;
        let win_rate = if total > 0 {
            format!("{:.0}%", row.win_count as f64 / total as f64 * 100.0)
        } else {
            "-".to_string()
        };
        let net = row.profit_realized - row.loss_realized;
        println!(
            "  {:12} {:>6} {:>6} ${:>9.2} {:>8}",
            row.date, row.opportunities_detected, row.trades_closed, net, win_rate
        );
    }

    Ok(())
}

fn print_open_positions(open_count: i64) {
    if open_count > 0 {
        output::field("Open positions", open_count);
    }
}

/// Execute `statistics export [--days N] [--output FILE]`.
pub fn execute_export(db_path: &Path, days: u32, output_path: Option<&Path>) -> Result<()> {
    let database_url = operator::sqlite_database_url(db_path);
    let (from, to, _) = date_range_history(days);
    let csv = export_daily_csv(&database_url, from, to)?;

    if output::is_json() {
        if let Some(path) = output_path {
            std::fs::write(path, &csv)?;
            println!(
                "{}",
                json!({
                    "command": "statistics.export",
                    "status": "written",
                    "days": days,
                    "from": from.to_string(),
                    "to": to.to_string(),
                    "path": path.display().to_string(),
                    "bytes": csv.len(),
                })
            );
        } else {
            println!(
                "{}",
                json!({
                    "command": "statistics.export",
                    "status": "stdout",
                    "days": days,
                    "from": from.to_string(),
                    "to": to.to_string(),
                    "csv": csv,
                })
            );
        }
        return Ok(());
    }

    if let Some(path) = output_path {
        std::fs::write(path, &csv)?;
        output::success("Statistics export complete");
        output::field("Days", days);
        output::field("Path", path.display());
    } else {
        print!("{csv}");
    }

    Ok(())
}

/// Execute `statistics prune [--days N]`.
pub fn execute_prune(db_path: &Path, retention_days: u32) -> Result<()> {
    let database_url = operator::sqlite_database_url(db_path);
    prune_old_records(&database_url, retention_days)?;

    if output::is_json() {
        println!(
            "{}",
            json!({
                "command": "statistics.prune",
                "status": "ok",
                "retention_days": retention_days,
                "note": "Aggregated daily statistics are preserved.",
            })
        );
        return Ok(());
    }

    output::success("Pruned historical opportunities and trades");
    output::field("Retention", format!("{retention_days} days"));
    println!("  Aggregated daily statistics are preserved.");
    Ok(())
}

fn summary_to_json(summary: &StatsSummary) -> serde_json::Value {
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

fn strategy_rows_to_json(rows: &[StrategyDailyStatsRow]) -> serde_json::Value {
    let payload: Vec<_> = rows
        .iter()
        .map(|row| {
            let total = row.win_count + row.loss_count;
            let win_rate = if total > 0 {
                Some(row.win_count as f64 / total as f64 * 100.0)
            } else {
                None
            };

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

fn daily_rows_to_json(rows: &[DailyStatsRow]) -> serde_json::Value {
    let payload: Vec<_> = rows
        .iter()
        .map(|row| {
            let total = row.win_count + row.loss_count;
            let win_rate = if total > 0 {
                Some(row.win_count as f64 / total as f64 * 100.0)
            } else {
                None
            };

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
