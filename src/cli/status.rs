//! Handler for the `status` command.

use std::path::Path;

use chrono::{Duration, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};

use crate::core::db::model::DailyStatsRow;
use crate::core::db::schema::{daily_stats, trades};

/// Execute the status command.
pub fn execute(db_path: &Path) {
    let version = env!("CARGO_PKG_VERSION");

    // Check if systemd service is running
    let (running, pid) = check_systemd_status();

    println!();
    println!("edgelord v{version}");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    if running {
        println!("Status:      ● running (pid {pid})");
    } else {
        println!("Status:      ○ stopped");
    }

    // Try to connect to database for stats
    if let Ok(pool) = connect(db_path) {
        display_db_stats(&pool);
    } else {
        println!();
        println!("Database:    not found ({db_path:?})");
        println!();
        println!("Run 'edgelord run' to start trading and create the database.");
    }

    println!();
}

fn check_systemd_status() -> (bool, String) {
    use std::process::Command;

    let output = Command::new("systemctl")
        .args(["is-active", "edgelord"])
        .output();

    let running = match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout).trim() == "active",
        Err(_) => false,
    };

    let pid = if running {
        Command::new("systemctl")
            .args(["show", "edgelord", "--property=MainPID", "--value"])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|p| p != "0" && !p.is_empty())
            .unwrap_or_default()
    } else {
        String::new()
    };

    (running, pid)
}

fn connect(db_path: &Path) -> Result<Pool<ConnectionManager<SqliteConnection>>, ()> {
    if !db_path.exists() {
        return Err(());
    }
    let db_url = format!("sqlite://{}", db_path.display());
    let manager = ConnectionManager::<SqliteConnection>::new(&db_url);
    Pool::builder().max_size(1).build(manager).map_err(|_| ())
}

fn display_db_stats(pool: &Pool<ConnectionManager<SqliteConnection>>) {
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return,
    };

    let today = Utc::now().date_naive();
    let week_ago = today - Duration::days(7);

    // Get today's stats
    let today_row: Option<DailyStatsRow> = daily_stats::table
        .filter(daily_stats::date.eq(today.to_string()))
        .first(&mut conn)
        .ok();

    // Get week's stats
    let week_rows: Vec<DailyStatsRow> = daily_stats::table
        .filter(daily_stats::date.ge(week_ago.to_string()))
        .filter(daily_stats::date.le(today.to_string()))
        .load(&mut conn)
        .unwrap_or_default();

    // Get open positions
    let open_positions: i64 = trades::table
        .filter(trades::status.eq("open"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    // Aggregate week stats
    let week_opps: i32 = week_rows.iter().map(|r| r.opportunities_detected).sum();
    let week_trades: i32 = week_rows.iter().map(|r| r.trades_closed).sum();
    let week_profit: f32 = week_rows
        .iter()
        .map(|r| r.profit_realized - r.loss_realized)
        .sum();
    let week_wins: i32 = week_rows.iter().map(|r| r.win_count).sum();
    let week_losses: i32 = week_rows.iter().map(|r| r.loss_count).sum();

    println!();

    // Today
    if let Some(row) = today_row {
        let net = row.profit_realized - row.loss_realized;
        let total = row.win_count + row.loss_count;
        let win_rate = if total > 0 {
            format!("{:.0}%", row.win_count as f64 / total as f64 * 100.0)
        } else {
            "-".to_string()
        };
        println!(
            "Today:       {} opps, {} trades, ${:.2} net, {} win rate",
            row.opportunities_detected, row.trades_closed, net, win_rate
        );
    } else {
        println!("Today:       no activity yet");
    }

    // Week
    if !week_rows.is_empty() {
        let win_rate = if week_wins + week_losses > 0 {
            format!(
                "{:.0}%",
                week_wins as f64 / (week_wins + week_losses) as f64 * 100.0
            )
        } else {
            "-".to_string()
        };
        println!(
            "This Week:   {} opps, {} trades, ${:.2} net, {} win rate",
            week_opps, week_trades, week_profit, win_rate
        );
    }

    // Open positions
    if open_positions > 0 {
        // Could calculate exposure here too
        println!();
        println!("Positions:   {} open", open_positions);
    }

    println!();
    println!("Use 'edgelord stats' for detailed breakdown");
}
