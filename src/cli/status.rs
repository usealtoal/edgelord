//! Handler for the `status` command.

use std::path::Path;

use chrono::Duration;

use crate::app::status;
use crate::cli::output;

/// Execute the status command.
pub fn execute(db_path: &Path) {
    let version = env!("CARGO_PKG_VERSION");

    // Check if systemd service is running
    let (running, pid) = check_systemd_status();

    output::section(&format!("edgelord v{version}"));
    if running {
        output::key_value("Status", format!("running (pid {pid})"));
    } else {
        output::key_value("Status", "stopped");
    }

    // Try to connect to database for stats
    if db_path.exists() {
        if let Ok(snapshot) = status::load_status(db_path) {
            display_db_stats(snapshot);
        } else {
            output::warn(&format!("Database error reading stats ({db_path:?})"));
        }
    } else {
        output::warn(&format!("Database not found ({db_path:?})"));
        output::note("Run `edgelord run` to start trading and create the database.");
    }
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

fn display_db_stats(snapshot: status::StatusSnapshot) {
    let today_row = snapshot.today;
    let week_rows = snapshot.week_rows;
    let open_positions = snapshot.open_positions;

    let today = chrono::Utc::now().date_naive();
    let week_ago = today - Duration::days(7);

    // Today
    if let Some(row) = today_row {
        output::section(&format!("Today ({today})"));
        output::key_value("Opportunities", row.opportunities_detected);
        output::key_value("Trades opened", row.trades_opened);
        output::key_value("Trades closed", row.trades_closed);
        output::key_value("Profit", format!("${:.2}", row.profit_realized));
        output::key_value("Loss", format!("${:.2}", row.loss_realized));
        output::key_value(
            "Net",
            format!("${:.2}", row.profit_realized - row.loss_realized),
        );
    } else {
        output::section(&format!("Today ({today})"));
        output::note("No data");
    }

    // Last 7 days
    let week_opps: i32 = week_rows.iter().map(|r| r.opportunities_detected).sum();
    let week_trades: i32 = week_rows.iter().map(|r| r.trades_closed).sum();
    let week_profit: f32 = week_rows
        .iter()
        .map(|r| r.profit_realized - r.loss_realized)
        .sum();
    let week_wins: i32 = week_rows.iter().map(|r| r.win_count).sum();
    let week_losses: i32 = week_rows.iter().map(|r| r.loss_count).sum();

    output::section(&format!("Last 7 Days ({week_ago} to {today})"));
    output::key_value("Opportunities", week_opps);
    output::key_value("Trades closed", week_trades);
    output::key_value("Net profit", format!("${:.2}", week_profit));
    if week_wins + week_losses > 0 {
        let win_rate = week_wins as f32 / (week_wins + week_losses) as f32 * 100.0;
        output::key_value("Win rate", format!("{win_rate:.1}%"));
    }

    output::key_value("Open positions", open_positions);
}
