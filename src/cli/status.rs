//! Handler for the `status` command.

use std::path::Path;

use chrono::Duration;

use crate::app::status::{self, RecentActivity};
use crate::cli::output;

/// Execute the status command.
pub fn execute(db_path: &Path) {
    // Check if systemd service is running
    let (running, pid, uptime) = check_systemd_status();

    output::header(env!("CARGO_PKG_VERSION"));

    // State
    if running {
        output::field("State", output::positive(format!("running (pid {pid})")));
    } else {
        output::field("State", output::muted("stopped"));
    }

    // Uptime
    if running && !uptime.is_empty() {
        output::field("Uptime", &uptime);
    }

    // Network - try to detect from environment or show default
    let network = detect_network();
    output::field("Network", &network);

    // Try to connect to database for stats
    if db_path.exists() {
        if let Ok(snapshot) = status::load_status(db_path) {
            display_db_stats(snapshot);
        } else {
            println!();
            output::warning(&format!("Database error reading stats ({db_path:?})"));
        }
    } else {
        println!();
        output::warning(&format!("Database not found ({db_path:?})"));
        println!("  Run `edgelord run` to start trading and create the database.");
    }
}

fn check_systemd_status() -> (bool, String, String) {
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

    // Get uptime if running
    let uptime = if running {
        Command::new("systemctl")
            .args([
                "show",
                "edgelord",
                "--property=ActiveEnterTimestamp",
                "--value",
            ])
            .output()
            .ok()
            .and_then(|o| {
                let ts = String::from_utf8_lossy(&o.stdout).trim().to_string();
                parse_uptime(&ts)
            })
            .unwrap_or_default()
    } else {
        String::new()
    };

    (running, pid, uptime)
}

/// Parse systemd timestamp and return human-readable uptime.
fn parse_uptime(timestamp: &str) -> Option<String> {
    // systemd format: "Wed 2026-02-21 10:30:00 UTC"
    use chrono::{DateTime, Utc};

    // Try to parse the timestamp
    let formats = [
        "%a %Y-%m-%d %H:%M:%S %Z",
        "%Y-%m-%d %H:%M:%S %Z",
        "%Y-%m-%dT%H:%M:%S%z",
    ];

    for fmt in &formats {
        if let Ok(dt) = DateTime::parse_from_str(timestamp, fmt) {
            let now = Utc::now();
            let duration = now.signed_duration_since(dt);
            return Some(format_duration(duration));
        }
    }
    None
}

/// Format a duration as human-readable string.
fn format_duration(duration: Duration) -> String {
    let total_secs = duration.num_seconds();
    if total_secs < 0 {
        return "0s".to_string();
    }

    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let mins = (total_secs % 3600) / 60;

    if days > 0 {
        format!("{}d {}h", days, hours)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else {
        format!("{}m", mins)
    }
}

/// Detect network from environment or default to mainnet.
fn detect_network() -> String {
    // Check common environment variables
    if let Ok(env) = std::env::var("EDGELORD_NETWORK") {
        return env;
    }
    if let Ok(chain) = std::env::var("CHAIN_ID") {
        return match chain.as_str() {
            "137" => "mainnet".to_string(),
            "80002" => "testnet (amoy)".to_string(),
            _ => format!("chain {}", chain),
        };
    }
    // Default
    "mainnet".to_string()
}

fn display_db_stats(snapshot: status::StatusSnapshot) {
    let today_row = snapshot.today;
    let open_positions = snapshot.open_positions;
    let distinct_markets = snapshot.distinct_markets;
    let current_exposure = snapshot.current_exposure;
    let recent_activity = snapshot.recent_activity;

    println!();

    // Exposure - we don't have max exposure from config here, so show current only
    if current_exposure > 0.0 {
        output::field("Exposure", format!("${:.2}", current_exposure));
    }

    // Positions
    if open_positions > 0 {
        output::field(
            "Positions",
            format!(
                "{} active across {} markets",
                open_positions, distinct_markets
            ),
        );
    } else {
        output::field("Positions", output::muted("none"));
    }

    // Today's stats
    output::section("Today");
    if let Some(row) = today_row {
        output::field("Opportunities", row.opportunities_detected);
        output::field("Executed", row.opportunities_executed);

        // Rejected with breakdown
        let rejected = row.opportunities_rejected;
        if rejected > 0 {
            // We don't have detailed breakdown in daily_stats, just show total
            output::field("Rejected", rejected);
        } else {
            output::field("Rejected", output::muted("0"));
        }

        // P&L
        let net = row.profit_realized - row.loss_realized;
        let pnl_display = if net >= 0.0 {
            output::positive(format!("+${:.2}", net))
        } else {
            output::negative(format!("-${:.2}", net.abs()))
        };
        output::field("P&L", pnl_display);
    } else {
        println!("  {}", output::muted("No data for today"));
    }

    // Recent activity
    if !recent_activity.is_empty() {
        output::section("Recent activity");
        for activity in recent_activity {
            match activity {
                RecentActivity::Executed {
                    timestamp,
                    profit,
                    market_description,
                } => {
                    let profit_str = if profit >= 0.0 {
                        output::positive(format!("+${:.2}", profit))
                    } else {
                        output::negative(format!("-${:.2}", profit.abs()))
                    };
                    output::executed(
                        &timestamp,
                        &format!("{}  \"{}\"", profit_str, market_description),
                    );
                }
                RecentActivity::Rejected { timestamp, reason } => {
                    output::rejected(&timestamp, &reason);
                }
            }
        }
    }
}
