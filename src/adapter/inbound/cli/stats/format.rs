//! Formatting and printing functions for statistics output.

use rust_decimal::Decimal;

use crate::adapter::inbound::cli::output;
use crate::domain::stats::StatsSummary;
use crate::error::Result;
use crate::port::inbound::operator::stats::{DailyStatsRecord, StrategyStatsRecord};

use super::aggregate::{aggregate_by_strategy, compute_percentage, compute_win_rate};

/// Print a statistics summary to stdout.
pub fn print_summary(summary: &StatsSummary, label: &str) -> Result<()> {
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

/// Print strategy breakdown table to stdout.
pub fn print_breakdown(rows: &[StrategyStatsRecord]) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }

    let by_strategy = aggregate_by_strategy(rows);

    output::section("By Strategy");
    println!(
        "  {:20} {:>8} {:>8} {:>10} {:>8}",
        "Strategy", "Opps", "Trades", "Profit", "Win %"
    );
    println!("  {:─<20} {:─>8} {:─>8} {:─>10} {:─>8}", "", "", "", "", "");

    for (name, stats_row) in &by_strategy {
        let win_rate = compute_win_rate(stats_row.win_count, stats_row.loss_count)
            .map(|r| format!("{r:.1}%"))
            .unwrap_or_else(|| "N/A".to_string());
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

/// Print daily breakdown table to stdout.
pub fn print_daily(rows: &[DailyStatsRecord]) -> Result<()> {
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
        let win_rate = compute_percentage(row.win_count, row.win_count + row.loss_count)
            .map(|r| format!("{r:.0}%"))
            .unwrap_or_else(|| "-".to_string());
        let net = row.profit_realized - row.loss_realized;
        println!(
            "  {:12} {:>6} {:>6} ${:>9.2} {:>8}",
            row.date, row.opportunities_detected, row.trades_closed, net, win_rate
        );
    }

    Ok(())
}

/// Print open positions count if non-zero.
pub fn print_open_positions(open_count: i64) {
    if open_count > 0 {
        output::field("Open positions", open_count);
    }
}
