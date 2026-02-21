//! Handler for the `statistics` command group.

use std::path::Path;

use rust_decimal::Decimal;

use crate::app::statistics;
use crate::cli::output;
use crate::error::Result;

/// Execute `statistics` (default: today).
pub fn execute_today(db_path: &Path) -> Result<()> {
    let (from, to, label) = statistics::date_range_today();
    let summary = statistics::load_summary(db_path, from, to)?;
    print_summary(&summary, &label)?;

    let rows = statistics::load_strategy_breakdown(db_path, from, to)?;
    print_strategy_breakdown(&rows)?;

    let open_positions = statistics::load_open_positions(db_path)?;
    print_open_positions(open_positions);

    Ok(())
}

/// Execute `statistics week`.
pub fn execute_week(db_path: &Path) -> Result<()> {
    let (from, to, label) = statistics::date_range_week();
    let summary = statistics::load_summary(db_path, from, to)?;
    print_summary(&summary, &label)?;

    let rows = statistics::load_strategy_breakdown(db_path, from, to)?;
    print_strategy_breakdown(&rows)?;

    Ok(())
}

/// Execute `statistics history [days]`.
pub fn execute_history(db_path: &Path, days: u32) -> Result<()> {
    let (from, to, label) = statistics::date_range_history(days);
    let summary = statistics::load_summary(db_path, from, to)?;
    print_summary(&summary, &label)?;

    let rows = statistics::load_daily_rows(db_path, from, to)?;
    print_daily_breakdown(&rows)?;

    Ok(())
}

fn print_summary(summary: &statistics::StatsSummary, label: &str) -> Result<()> {
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
                "✓"
            } else {
                "✗"
            }
        ),
    );
    output::field("Volume", format!("${:.2}", summary.total_volume));

    Ok(())
}

fn print_strategy_breakdown(rows: &[statistics::StrategyDailyStatsRow]) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }

    let mut by_strategy: std::collections::HashMap<String, statistics::StrategyDailyStatsRow> =
        std::collections::HashMap::new();

    for row in rows {
        let entry = by_strategy.entry(row.strategy.clone()).or_insert_with(|| {
            statistics::StrategyDailyStatsRow {
                date: String::new(),
                strategy: row.strategy.clone(),
                ..Default::default()
            }
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

fn print_daily_breakdown(rows: &[statistics::DailyStatsRow]) -> Result<()> {
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
    let (from, to, _) = statistics::date_range_history(days);
    let csv = statistics::export_daily_csv(db_path, from, to)?;

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
    statistics::prune_old_records(db_path, retention_days)?;
    output::success("Pruned historical opportunities and trades");
    output::field("Retention", format!("{retention_days} days"));
    println!("  Aggregated daily statistics are preserved.");
    Ok(())
}
