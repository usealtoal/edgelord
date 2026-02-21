use std::path::Path;

use chrono::{Duration, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};

use crate::adapters::stores::db::model::{DailyStatsRow, OpportunityRow, TradeRow};
use crate::adapters::stores::db::schema::{daily_stats, opportunities, trades};
use crate::error::{ConfigError, Error, Result};

fn connect(db_path: &Path) -> Result<Pool<ConnectionManager<SqliteConnection>>> {
    let db_url = format!("sqlite://{}", db_path.display());
    let manager = ConnectionManager::<SqliteConnection>::new(&db_url);
    Pool::builder()
        .max_size(1)
        .build(manager)
        .map_err(|e| Error::Config(ConfigError::Other(e.to_string())))
}

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

pub struct StatusSnapshot {
    pub today: Option<DailyStatsRow>,
    pub week_rows: Vec<DailyStatsRow>,
    pub open_positions: i64,
    pub distinct_markets: i64,
    pub current_exposure: f32,
    pub recent_activity: Vec<RecentActivity>,
}

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
        .flat_map(|t| {
            serde_json::from_str::<Vec<String>>(&t.market_ids).unwrap_or_default()
        })
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
            // Extract time portion for display
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
    // Try to parse ISO format: 2026-01-01T12:34:56Z or 2026-01-01 12:34:56
    if let Some(t_pos) = timestamp.find('T') {
        let time_part = &timestamp[t_pos + 1..];
        // Take first 8 chars (HH:MM:SS)
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
            // Shorten the ID for display
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
