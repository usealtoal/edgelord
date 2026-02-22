//! Handler for the `status` command.

use std::path::Path;

use crate::adapter::inbound::cli::{operator, output};
use crate::port::inbound::operator::status::{RecentActivity, StatusSnapshot};
use serde_json::json;

/// Execute the status command.
pub fn execute(db_path: &Path, config_path: Option<&Path>) {
    if output::is_quiet() && !output::is_json() {
        return;
    }

    let service = operator::operator();

    let network = config_path
        .and_then(|path| operator::read_config_toml(path).ok())
        .and_then(|config_toml| service.network_label(&config_toml).ok())
        .unwrap_or_else(|| "unknown".to_string());

    if output::is_json() {
        let payload = if db_path.exists() {
            let database_url = operator::sqlite_database_url(db_path);
            match service.load_status(&database_url) {
                Ok(snapshot) => json!({
                    "command": "status",
                    "network": network,
                    "database": db_path.display().to_string(),
                    "status": "ok",
                    "snapshot": snapshot_to_json(&snapshot),
                }),
                Err(error) => json!({
                    "command": "status",
                    "network": network,
                    "database": db_path.display().to_string(),
                    "status": "error",
                    "error": error.to_string(),
                }),
            }
        } else {
            json!({
                "command": "status",
                "network": network,
                "database": db_path.display().to_string(),
                "status": "missing_database",
            })
        };
        println!("{payload}");
        return;
    }

    output::header(env!("CARGO_PKG_VERSION"));
    output::field("Network", &network);

    if db_path.exists() {
        let database_url = operator::sqlite_database_url(db_path);
        if let Ok(snapshot) = service.load_status(&database_url) {
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

fn snapshot_to_json(snapshot: &StatusSnapshot) -> serde_json::Value {
    let today = snapshot.today.as_ref().map(|row| {
        json!({
            "opportunities_detected": row.opportunities_detected,
            "opportunities_executed": row.opportunities_executed,
            "opportunities_rejected": row.opportunities_rejected,
            "profit_realized": row.profit_realized,
            "loss_realized": row.loss_realized,
        })
    });

    let recent_activity: Vec<_> = snapshot
        .recent_activity
        .iter()
        .map(|item| match item {
            RecentActivity::Executed {
                timestamp,
                profit,
                market_description,
            } => json!({
                "type": "executed",
                "timestamp": timestamp,
                "profit": profit,
                "market_description": market_description,
            }),
            RecentActivity::Rejected { timestamp, reason } => json!({
                "type": "rejected",
                "timestamp": timestamp,
                "reason": reason,
            }),
        })
        .collect();

    json!({
        "today": today,
        "open_positions": snapshot.open_positions,
        "distinct_markets": snapshot.distinct_markets,
        "current_exposure": snapshot.current_exposure,
        "recent_activity": recent_activity,
    })
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
