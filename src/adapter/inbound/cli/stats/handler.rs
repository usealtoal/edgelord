//! Handler for the `statistics` command group.
//!
//! This module orchestrates data loading, aggregation, and formatting
//! for statistics CLI commands.

use std::path::Path;

use chrono::NaiveDate;
use serde_json::json;

use crate::adapter::inbound::cli::{operator, output};
use crate::domain::stats::StatsSummary;
use crate::error::Result;
use crate::port::inbound::operator::stats::{DailyStatsRecord, StrategyStatsRecord};

use super::format::{print_breakdown, print_daily, print_open_positions, print_summary};
use super::json::{daily_rows_to_json, strategy_rows_to_json, summary_to_json};
use super::range::DateRange;

// Data loading helpers - delegate to operator

fn load_summary(database_url: &str, from: NaiveDate, to: NaiveDate) -> Result<StatsSummary> {
    operator::operator().load_summary(database_url, from, to)
}

fn load_strategy_breakdown(
    database_url: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<StrategyStatsRecord>> {
    operator::operator().load_strategy_breakdown(database_url, from, to)
}

fn load_open_positions(database_url: &str) -> Result<i64> {
    operator::operator().load_open_positions(database_url)
}

fn load_daily_rows(
    database_url: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<DailyStatsRecord>> {
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
    let range = DateRange::today();
    let summary = load_summary(&database_url, range.start, range.end)?;
    let rows = load_strategy_breakdown(&database_url, range.start, range.end)?;
    let open_positions = load_open_positions(&database_url)?;

    if output::is_json() {
        println!(
            "{}",
            json!({
                "command": "statistics.today",
                "label": range.label,
                "summary": summary_to_json(&summary),
                "strategy_breakdown": strategy_rows_to_json(&rows),
                "open_positions": open_positions,
            })
        );
        return Ok(());
    }

    print_summary(&summary, &range.label)?;
    print_breakdown(&rows)?;
    print_open_positions(open_positions);

    Ok(())
}

/// Execute `statistics week`.
pub fn execute_week(db_path: &Path) -> Result<()> {
    if output::is_quiet() && !output::is_json() {
        return Ok(());
    }

    let database_url = operator::sqlite_database_url(db_path);
    let range = DateRange::week();
    let summary = load_summary(&database_url, range.start, range.end)?;
    let rows = load_strategy_breakdown(&database_url, range.start, range.end)?;

    if output::is_json() {
        println!(
            "{}",
            json!({
                "command": "statistics.week",
                "label": range.label,
                "summary": summary_to_json(&summary),
                "strategy_breakdown": strategy_rows_to_json(&rows),
            })
        );
        return Ok(());
    }

    print_summary(&summary, &range.label)?;
    print_breakdown(&rows)?;

    Ok(())
}

/// Execute `statistics history [days]`.
pub fn execute_history(db_path: &Path, days: u32) -> Result<()> {
    if output::is_quiet() && !output::is_json() {
        return Ok(());
    }

    let database_url = operator::sqlite_database_url(db_path);
    let range = DateRange::history(days);
    let summary = load_summary(&database_url, range.start, range.end)?;
    let rows = load_daily_rows(&database_url, range.start, range.end)?;

    if output::is_json() {
        println!(
            "{}",
            json!({
                "command": "statistics.history",
                "label": range.label,
                "days": days,
                "from": range.start.to_string(),
                "to": range.end.to_string(),
                "summary": summary_to_json(&summary),
                "daily_breakdown": daily_rows_to_json(&rows),
            })
        );
        return Ok(());
    }

    print_summary(&summary, &range.label)?;
    print_daily(&rows)?;

    Ok(())
}

/// Execute `statistics export [--days N] [--output FILE]`.
pub fn execute_export(db_path: &Path, days: u32, output_path: Option<&Path>) -> Result<()> {
    let database_url = operator::sqlite_database_url(db_path);
    let range = DateRange::history(days);
    let csv = export_daily_csv(&database_url, range.start, range.end)?;

    if output::is_json() {
        if let Some(path) = output_path {
            std::fs::write(path, &csv)?;
            println!(
                "{}",
                json!({
                    "command": "statistics.export",
                    "status": "written",
                    "days": days,
                    "from": range.start.to_string(),
                    "to": range.end.to_string(),
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
                    "from": range.start.to_string(),
                    "to": range.end.to_string(),
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
