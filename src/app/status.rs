use std::path::Path;

use chrono::{Duration, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};

use crate::core::db::model::DailyStatsRow;
use crate::core::db::schema::{daily_stats, trades};
use crate::error::{ConfigError, Error, Result};

fn connect(db_path: &Path) -> Result<Pool<ConnectionManager<SqliteConnection>>> {
    let db_url = format!("sqlite://{}", db_path.display());
    let manager = ConnectionManager::<SqliteConnection>::new(&db_url);
    Pool::builder()
        .max_size(1)
        .build(manager)
        .map_err(|e| Error::Config(ConfigError::Other(e.to_string())))
}

pub struct StatusSnapshot {
    pub today: Option<DailyStatsRow>,
    pub week_rows: Vec<DailyStatsRow>,
    pub open_positions: i64,
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

    let open_positions: i64 = trades::table
        .filter(trades::status.eq("open"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    Ok(StatusSnapshot {
        today: today_row,
        week_rows,
        open_positions,
    })
}
