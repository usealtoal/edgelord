//! Handler for the `statistics` command group.

use std::path::Path;

use chrono::{Duration, NaiveDate, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use rust_decimal::Decimal;

use crate::adapter::statistic::{
    export_daily_csv as export_csv_impl, f32_to_decimal, StatsRecorder, StatsSummary,
};
use crate::adapter::store::db::model::{DailyStatsRow, StrategyDailyStatsRow};
use crate::adapter::store::db::schema::{daily_stats, strategy_daily_stats, trades};
use crate::cli::output;
use crate::error::{ConfigError, Error, Result};

// ============================================================================
// Data access
// ============================================================================

fn connect(db_path: &Path) -> Result<Pool<ConnectionManager<SqliteConnection>>> {
    let db_url = format!("sqlite://{}", db_path.display());
    let manager = ConnectionManager::<SqliteConnection>::new(&db_url);
    Pool::builder()
        .max_size(1)
        .build(manager)
        .map_err(|e| Error::Config(ConfigError::Other(e.to_string())))
}

fn load_summary(db_path: &Path, from: NaiveDate, to: NaiveDate) -> Result<StatsSummary> {
    let pool = connect(db_path)?;
    let mut conn = pool
        .get()
        .map_err(|e| Error::Config(ConfigError::Other(e.to_string())))?;

    let rows: Vec<DailyStatsRow> = daily_stats::table
        .filter(daily_stats::date.ge(from.to_string()))
        .filter(daily_stats::date.le(to.to_string()))
        .load(&mut conn)
        .unwrap_or_default();

    let mut summary = StatsSummary::default();
    for row in rows {
        summary.opportunities_detected += i64::from(row.opportunities_detected);
        summary.opportunities_executed += i64::from(row.opportunities_executed);
        summary.opportunities_rejected += i64::from(row.opportunities_rejected);
        summary.trades_opened += i64::from(row.trades_opened);
        summary.trades_closed += i64::from(row.trades_closed);
        summary.profit_realized += f32_to_decimal(row.profit_realized);
        summary.loss_realized += f32_to_decimal(row.loss_realized);
        summary.win_count += i64::from(row.win_count);
        summary.loss_count += i64::from(row.loss_count);
        summary.total_volume += f32_to_decimal(row.total_volume);
    }

    Ok(summary)
}

fn load_strategy_breakdown(
    db_path: &Path,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<StrategyDailyStatsRow>> {
    let pool = connect(db_path)?;
    let mut conn = pool
        .get()
        .map_err(|e| Error::Config(ConfigError::Other(e.to_string())))?;

    let rows: Vec<StrategyDailyStatsRow> = strategy_daily_stats::table
        .filter(strategy_daily_stats::date.ge(from.to_string()))
        .filter(strategy_daily_stats::date.le(to.to_string()))
        .load(&mut conn)
        .unwrap_or_default();

    Ok(rows)
}

fn load_open_positions(db_path: &Path) -> Result<i64> {
    let pool = connect(db_path)?;
    let mut conn = pool
        .get()
        .map_err(|e| Error::Config(ConfigError::Other(e.to_string())))?;

    let open_count: i64 = trades::table
        .filter(trades::status.eq("open"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    Ok(open_count)
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

fn load_daily_rows(db_path: &Path, from: NaiveDate, to: NaiveDate) -> Result<Vec<DailyStatsRow>> {
    let pool = connect(db_path)?;
    let mut conn = pool
        .get()
        .map_err(|e| Error::Config(ConfigError::Other(e.to_string())))?;

    let rows: Vec<DailyStatsRow> = daily_stats::table
        .filter(daily_stats::date.ge(from.to_string()))
        .filter(daily_stats::date.le(to.to_string()))
        .order(daily_stats::date.desc())
        .load(&mut conn)
        .unwrap_or_default();

    Ok(rows)
}

fn export_daily_csv(db_path: &Path, from: NaiveDate, to: NaiveDate) -> Result<String> {
    let pool = connect(db_path)?;
    Ok(export_csv_impl(&pool, from, to))
}

fn prune_old_records(db_path: &Path, retention_days: u32) -> Result<()> {
    let pool = connect(db_path)?;
    let recorder = StatsRecorder::new(pool);
    recorder.prune_old_records(retention_days);
    Ok(())
}

// ============================================================================
// CLI commands
// ============================================================================

/// Execute `statistics` (default: today).
pub fn execute_today(db_path: &Path) -> Result<()> {
    let (from, to, label) = date_range_today();
    let summary = load_summary(db_path, from, to)?;
    print_summary(&summary, &label)?;

    let rows = load_strategy_breakdown(db_path, from, to)?;
    print_strategy_breakdown(&rows)?;

    let open_positions = load_open_positions(db_path)?;
    print_open_positions(open_positions);

    Ok(())
}

/// Execute `statistics week`.
pub fn execute_week(db_path: &Path) -> Result<()> {
    let (from, to, label) = date_range_week();
    let summary = load_summary(db_path, from, to)?;
    print_summary(&summary, &label)?;

    let rows = load_strategy_breakdown(db_path, from, to)?;
    print_strategy_breakdown(&rows)?;

    Ok(())
}

/// Execute `statistics history [days]`.
pub fn execute_history(db_path: &Path, days: u32) -> Result<()> {
    let (from, to, label) = date_range_history(days);
    let summary = load_summary(db_path, from, to)?;
    print_summary(&summary, &label)?;

    let rows = load_daily_rows(db_path, from, to)?;
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
                    date: String::new(),
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
    let (from, to, _) = date_range_history(days);
    let csv = export_daily_csv(db_path, from, to)?;

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
    prune_old_records(db_path, retention_days)?;
    output::success("Pruned historical opportunities and trades");
    output::field("Retention", format!("{retention_days} days"));
    println!("  Aggregated daily statistics are preserved.");
    Ok(())
}
