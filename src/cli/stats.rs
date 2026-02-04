//! Handler for the `stats` command group.

use std::path::Path;

use chrono::{Duration, NaiveDate, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use rust_decimal::Decimal;

use crate::core::db::model::{DailyStatsRow, StrategyDailyStatsRow};
use crate::core::db::schema::{daily_stats, strategy_daily_stats, trades};
use crate::core::service::stats::StatsSummary;
use crate::error::Result;

/// Execute `stats` (default: today).
pub fn execute_today(db_path: &Path) -> Result<()> {
    let pool = connect(db_path)?;
    let today = Utc::now().date_naive();
    print_summary(&pool, today, today, "Today")?;
    print_strategy_breakdown(&pool, today, today)?;
    print_open_positions(&pool)?;
    Ok(())
}

/// Execute `stats week`.
pub fn execute_week(db_path: &Path) -> Result<()> {
    let pool = connect(db_path)?;
    let today = Utc::now().date_naive();
    let week_ago = today - Duration::days(7);
    print_summary(&pool, week_ago, today, "Last 7 Days")?;
    print_strategy_breakdown(&pool, week_ago, today)?;
    Ok(())
}

/// Execute `stats history [days]`.
pub fn execute_history(db_path: &Path, days: u32) -> Result<()> {
    let pool = connect(db_path)?;
    let today = Utc::now().date_naive();
    let start = today - Duration::days(i64::from(days));
    let label = format!("Last {days} Days");
    print_summary(&pool, start, today, &label)?;
    print_daily_breakdown(&pool, start, today)?;
    Ok(())
}

fn connect(db_path: &Path) -> Result<Pool<ConnectionManager<SqliteConnection>>> {
    let db_url = format!("sqlite://{}", db_path.display());
    let manager = ConnectionManager::<SqliteConnection>::new(&db_url);
    let pool = Pool::builder()
        .max_size(1)
        .build(manager)
        .map_err(|e| crate::error::Error::Config(crate::error::ConfigError::Other(e.to_string())))?;
    Ok(pool)
}

fn print_summary(
    pool: &Pool<ConnectionManager<SqliteConnection>>,
    from: NaiveDate,
    to: NaiveDate,
    label: &str,
) -> Result<()> {
    let mut conn = pool.get().map_err(|e| {
        crate::error::Error::Config(crate::error::ConfigError::Other(e.to_string()))
    })?;

    let rows: Vec<DailyStatsRow> = daily_stats::table
        .filter(daily_stats::date.ge(from.to_string()))
        .filter(daily_stats::date.le(to.to_string()))
        .load(&mut conn)
        .unwrap_or_default();

    let summary = aggregate_rows(&rows);

    println!();
    println!("═══════════════════════════════════════════════════════════");
    println!("  {label}");
    println!("═══════════════════════════════════════════════════════════");
    println!();
    println!("  Opportunities");
    println!("  ─────────────────────────────────────────────────────────");
    println!(
        "    Detected:     {:>8}",
        summary.opportunities_detected
    );
    println!(
        "    Executed:     {:>8}    ({:.1}%)",
        summary.opportunities_executed,
        if summary.opportunities_detected > 0 {
            summary.opportunities_executed as f64 / summary.opportunities_detected as f64 * 100.0
        } else {
            0.0
        }
    );
    println!(
        "    Rejected:     {:>8}",
        summary.opportunities_rejected
    );
    println!();
    println!("  Trades");
    println!("  ─────────────────────────────────────────────────────────");
    println!("    Opened:       {:>8}", summary.trades_opened);
    println!("    Closed:       {:>8}", summary.trades_closed);
    println!(
        "    Win Rate:     {:>7}%",
        summary
            .win_rate()
            .map(|r| format!("{r:.1}"))
            .unwrap_or_else(|| "N/A".to_string())
    );
    println!();
    println!("  Profit/Loss");
    println!("  ─────────────────────────────────────────────────────────");
    println!("    Profit:       ${:>8.2}", summary.profit_realized);
    println!("    Loss:         ${:>8.2}", summary.loss_realized);
    println!(
        "    Net:          ${:>8.2}  {}",
        summary.net_profit(),
        if summary.net_profit() >= Decimal::ZERO {
            "✓"
        } else {
            "✗"
        }
    );
    println!();
    println!("    Volume:       ${:>8.2}", summary.total_volume);
    println!();

    Ok(())
}

fn print_strategy_breakdown(
    pool: &Pool<ConnectionManager<SqliteConnection>>,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<()> {
    let mut conn = pool.get().map_err(|e| {
        crate::error::Error::Config(crate::error::ConfigError::Other(e.to_string()))
    })?;

    let rows: Vec<StrategyDailyStatsRow> = strategy_daily_stats::table
        .filter(strategy_daily_stats::date.ge(from.to_string()))
        .filter(strategy_daily_stats::date.le(to.to_string()))
        .load(&mut conn)
        .unwrap_or_default();

    if rows.is_empty() {
        return Ok(());
    }

    // Group by strategy
    let mut by_strategy: std::collections::HashMap<String, StrategyDailyStatsRow> =
        std::collections::HashMap::new();

    for row in rows {
        let entry = by_strategy.entry(row.strategy.clone()).or_insert_with(|| {
            StrategyDailyStatsRow {
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

    println!("  By Strategy");
    println!("  ─────────────────────────────────────────────────────────");
    println!(
        "    {:20} {:>8} {:>8} {:>10} {:>8}",
        "Strategy", "Opps", "Trades", "Profit", "Win %"
    );
    println!("    {:─<20} {:─>8} {:─>8} {:─>10} {:─>8}", "", "", "", "", "");

    for (name, stats) in &by_strategy {
        let total = stats.win_count + stats.loss_count;
        let win_rate = if total > 0 {
            format!("{:.1}%", stats.win_count as f64 / total as f64 * 100.0)
        } else {
            "N/A".to_string()
        };
        println!(
            "    {:20} {:>8} {:>8} ${:>9.2} {:>8}",
            name,
            stats.opportunities_detected,
            stats.trades_closed,
            stats.profit_realized,
            win_rate
        );
    }
    println!();

    Ok(())
}

fn print_daily_breakdown(
    pool: &Pool<ConnectionManager<SqliteConnection>>,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<()> {
    let mut conn = pool.get().map_err(|e| {
        crate::error::Error::Config(crate::error::ConfigError::Other(e.to_string()))
    })?;

    let rows: Vec<DailyStatsRow> = daily_stats::table
        .filter(daily_stats::date.ge(from.to_string()))
        .filter(daily_stats::date.le(to.to_string()))
        .order(daily_stats::date.desc())
        .load(&mut conn)
        .unwrap_or_default();

    if rows.is_empty() {
        println!("  No data for this period.");
        println!();
        return Ok(());
    }

    println!("  Daily Breakdown");
    println!("  ─────────────────────────────────────────────────────────");
    println!(
        "    {:12} {:>6} {:>6} {:>10} {:>8}",
        "Date", "Opps", "Trades", "Net P/L", "Win %"
    );
    println!("    {:─<12} {:─>6} {:─>6} {:─>10} {:─>8}", "", "", "", "", "");

    for row in rows {
        let total = row.win_count + row.loss_count;
        let win_rate = if total > 0 {
            format!("{:.0}%", row.win_count as f64 / total as f64 * 100.0)
        } else {
            "-".to_string()
        };
        let net = row.profit_realized - row.loss_realized;
        println!(
            "    {:12} {:>6} {:>6} ${:>9.2} {:>8}",
            row.date, row.opportunities_detected, row.trades_closed, net, win_rate
        );
    }
    println!();

    Ok(())
}

fn print_open_positions(pool: &Pool<ConnectionManager<SqliteConnection>>) -> Result<()> {
    let mut conn = pool.get().map_err(|e| {
        crate::error::Error::Config(crate::error::ConfigError::Other(e.to_string()))
    })?;

    let open_count: i64 = trades::table
        .filter(trades::status.eq("open"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    if open_count > 0 {
        println!("  Open Positions: {open_count}");
        println!();
    }

    Ok(())
}

/// Execute `stats export [--days N] [--output FILE]`.
pub fn execute_export(db_path: &Path, days: u32, output: Option<&Path>) -> Result<()> {
    let pool = connect(db_path)?;
    let today = Utc::now().date_naive();
    let start = today - Duration::days(i64::from(days));

    let stats_recorder = crate::core::service::stats::StatsRecorder::new(pool);
    let csv = stats_recorder.export_daily_csv(start, today);

    if let Some(path) = output {
        std::fs::write(path, &csv)?;
        println!("Exported {} days of stats to {}", days, path.display());
    } else {
        print!("{csv}");
    }

    Ok(())
}

/// Execute `stats prune [--days N]`.
pub fn execute_prune(db_path: &Path, retention_days: u32) -> Result<()> {
    let pool = connect(db_path)?;
    let stats_recorder = crate::core::service::stats::StatsRecorder::new(pool);
    stats_recorder.prune_old_records(retention_days);
    println!(
        "Pruned opportunities and trades older than {} days",
        retention_days
    );
    println!("Note: Aggregated daily stats are preserved.");
    Ok(())
}

fn aggregate_rows(rows: &[DailyStatsRow]) -> StatsSummary {
    use rust_decimal::prelude::FromPrimitive;

    let mut summary = StatsSummary::default();
    for row in rows {
        summary.opportunities_detected += i64::from(row.opportunities_detected);
        summary.opportunities_executed += i64::from(row.opportunities_executed);
        summary.opportunities_rejected += i64::from(row.opportunities_rejected);
        summary.trades_opened += i64::from(row.trades_opened);
        summary.trades_closed += i64::from(row.trades_closed);
        summary.profit_realized += Decimal::from_f32(row.profit_realized).unwrap_or_default();
        summary.loss_realized += Decimal::from_f32(row.loss_realized).unwrap_or_default();
        summary.win_count += i64::from(row.win_count);
        summary.loss_count += i64::from(row.loss_count);
        summary.total_volume += Decimal::from_f32(row.total_volume).unwrap_or_default();
    }
    summary
}
