//! Handler for the `status` command.

use std::path::Path;

use chrono::{Duration, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};

use crate::adapter::cache::db::model::{DailyStatsRow, OpportunityRow, TradeRow};
use crate::adapter::cache::db::schema::{daily_stats, opportunities, trades};
use crate::cli::output;
use crate::error::{ConfigError, Error, Result};

// ============================================================================
// Data types
// ============================================================================

/// A recent activity item for display.
#[derive(Debug, Clone)]
pub enum RecentActivity {
    Executed {
        timestamp: String,
        profit: f32,
        market_description: String,
    },
    Rejected {
        timestamp: String,
        reason: String,
    },
}

/// Snapshot of current status from the database.
pub struct StatusSnapshot {
    pub today: Option<DailyStatsRow>,
    pub week_rows: Vec<DailyStatsRow>,
    pub open_positions: i64,
    pub distinct_markets: i64,
    pub current_exposure: f32,
    pub recent_activity: Vec<RecentActivity>,
}

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

/// Load status snapshot from the database.
pub fn load_status(db_path: &Path) -> Result<StatusSnapshot> {
    let pool = connect(db_path)?;
    let mut conn = pool
        .get()
        .map_err(|e| Error::Config(ConfigError::Other(e.to_string())))?;

    let today = Utc::now().date_naive();
    let week_ago = today - Duration::days(7);

    let today_row: Option<DailyStatsRow> = daily_stats::table
        .filter(daily_stats::date.eq(today.to_string()))
        .first(&mut conn)
        .ok();

    let week_rows: Vec<DailyStatsRow> = daily_stats::table
        .filter(daily_stats::date.ge(week_ago.to_string()))
        .filter(daily_stats::date.le(today.to_string()))
        .load(&mut conn)
        .unwrap_or_default();

    // Get open trades
    let open_trades: Vec<TradeRow> = trades::table
        .filter(trades::status.eq("open"))
        .load(&mut conn)
        .unwrap_or_default();

    let open_positions = open_trades.len() as i64;

    // Count distinct markets from open trades
    let distinct_markets = open_trades
        .iter()
        .flat_map(|t| serde_json::from_str::<Vec<String>>(&t.market_ids).unwrap_or_default())
        .collect::<std::collections::HashSet<_>>()
        .len() as i64;

    // Calculate current exposure from open trades
    let current_exposure: f32 = open_trades.iter().map(|t| t.size).sum();

    // Get recent activity (last 10 items)
    let recent_trades: Vec<TradeRow> = trades::table
        .filter(trades::status.eq("closed"))
        .order(trades::closed_at.desc())
        .limit(5)
        .load(&mut conn)
        .unwrap_or_default();

    let recent_rejected: Vec<OpportunityRow> = opportunities::table
        .filter(opportunities::executed.eq(0))
        .filter(opportunities::rejected_reason.is_not_null())
        .order(opportunities::detected_at.desc())
        .limit(5)
        .load(&mut conn)
        .unwrap_or_default();

    // Combine and sort recent activity
    let mut recent_activity: Vec<RecentActivity> = Vec::new();

    for trade in recent_trades {
        if let Some(closed_at) = &trade.closed_at {
            let timestamp = extract_time(closed_at);
            recent_activity.push(RecentActivity::Executed {
                timestamp,
                profit: trade.realized_profit.unwrap_or(0.0),
                market_description: extract_market_description(&trade.market_ids),
            });
        }
    }

    for opp in recent_rejected {
        let timestamp = extract_time(&opp.detected_at);
        recent_activity.push(RecentActivity::Rejected {
            timestamp,
            reason: opp.rejected_reason.unwrap_or_else(|| "unknown".to_string()),
        });
    }

    // Sort by timestamp descending and take top 5
    recent_activity.sort_by(|a, b| {
        let ts_a = match a {
            RecentActivity::Executed { timestamp, .. } => timestamp,
            RecentActivity::Rejected { timestamp, .. } => timestamp,
        };
        let ts_b = match b {
            RecentActivity::Executed { timestamp, .. } => timestamp,
            RecentActivity::Rejected { timestamp, .. } => timestamp,
        };
        ts_b.cmp(ts_a)
    });
    recent_activity.truncate(5);

    Ok(StatusSnapshot {
        today: today_row,
        week_rows,
        open_positions,
        distinct_markets,
        current_exposure,
        recent_activity,
    })
}

/// Extract time portion from ISO timestamp (HH:MM:SS).
fn extract_time(timestamp: &str) -> String {
    if let Some(t_pos) = timestamp.find('T') {
        let time_part = &timestamp[t_pos + 1..];
        time_part.chars().take(8).collect()
    } else if let Some(space_pos) = timestamp.find(' ') {
        let time_part = &timestamp[space_pos + 1..];
        time_part.chars().take(8).collect()
    } else {
        timestamp.to_string()
    }
}

/// Extract a short market description from market_ids JSON.
fn extract_market_description(market_ids_json: &str) -> String {
    if let Ok(ids) = serde_json::from_str::<Vec<String>>(market_ids_json) {
        if ids.is_empty() {
            "unknown market".to_string()
        } else if ids.len() == 1 {
            let id = &ids[0];
            if id.len() > 16 {
                format!("{}...", &id[..12])
            } else {
                id.clone()
            }
        } else {
            format!("{} markets", ids.len())
        }
    } else {
        "unknown market".to_string()
    }
}

// ============================================================================
// CLI command
// ============================================================================

/// Execute the status command.
pub fn execute(db_path: &Path) {
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

    // Network
    let network = detect_network();
    output::field("Network", &network);

    // Try to connect to database for stats
    if db_path.exists() {
        if let Ok(snapshot) = load_status(db_path) {
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

fn parse_uptime(timestamp: &str) -> Option<String> {
    use chrono::DateTime;

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

fn detect_network() -> String {
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
    "mainnet".to_string()
}

fn display_db_stats(snapshot: StatusSnapshot) {
    let today_row = snapshot.today;
    let open_positions = snapshot.open_positions;
    let distinct_markets = snapshot.distinct_markets;
    let current_exposure = snapshot.current_exposure;
    let recent_activity = snapshot.recent_activity;

    println!();

    if current_exposure > 0.0 {
        output::field("Exposure", format!("${:.2}", current_exposure));
    }

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

    output::section("Today");
    if let Some(row) = today_row {
        output::field("Opportunities", row.opportunities_detected);
        output::field("Executed", row.opportunities_executed);

        let rejected = row.opportunities_rejected;
        if rejected > 0 {
            output::field("Rejected", rejected);
        } else {
            output::field("Rejected", output::muted("0"));
        }

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
