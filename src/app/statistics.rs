use std::path::Path;

use chrono::{Duration, NaiveDate, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};

use crate::adapters::statistics::f32_to_decimal;
use crate::adapters::stores::db::schema::{daily_stats, strategy_daily_stats, trades};
use crate::error::{ConfigError, Error, Result};

// Re-export types for public use
pub use crate::adapters::statistics::{StatsRecorder, StatsSummary};
pub use crate::adapters::stores::db::model::{DailyStatsRow, StrategyDailyStatsRow};

fn connect(db_path: &Path) -> Result<Pool<ConnectionManager<SqliteConnection>>> {
    let db_url = format!("sqlite://{}", db_path.display());
    let manager = ConnectionManager::<SqliteConnection>::new(&db_url);
    Pool::builder()
        .max_size(1)
        .build(manager)
        .map_err(|e| Error::Config(ConfigError::Other(e.to_string())))
}

pub fn load_summary(db_path: &Path, from: NaiveDate, to: NaiveDate) -> Result<StatsSummary> {
    let pool = connect(db_path)?;
    let mut conn = pool
        .get()
        .map_err(|e| Error::Config(ConfigError::Other(e.to_string())))?;

    let rows: Vec<DailyStatsRow> = daily_stats::table
        .filter(daily_stats::date.ge(from.to_string()))
        .filter(daily_stats::date.le(to.to_string()))
        .load(&mut conn)
        .unwrap_or_default();

    let mut summary = StatsSummary::default();
    for row in rows {
        summary.opportunities_detected += i64::from(row.opportunities_detected);
        summary.opportunities_executed += i64::from(row.opportunities_executed);
        summary.opportunities_rejected += i64::from(row.opportunities_rejected);
        summary.trades_opened += i64::from(row.trades_opened);
        summary.trades_closed += i64::from(row.trades_closed);
        summary.profit_realized += f32_to_decimal(row.profit_realized);
        summary.loss_realized += f32_to_decimal(row.loss_realized);
        summary.win_count += i64::from(row.win_count);
        summary.loss_count += i64::from(row.loss_count);
        summary.total_volume += f32_to_decimal(row.total_volume);
    }

    Ok(summary)
}

pub fn load_strategy_breakdown(
    db_path: &Path,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<StrategyDailyStatsRow>> {
    let pool = connect(db_path)?;
    let mut conn = pool
        .get()
        .map_err(|e| Error::Config(ConfigError::Other(e.to_string())))?;

    let rows: Vec<StrategyDailyStatsRow> = strategy_daily_stats::table
        .filter(strategy_daily_stats::date.ge(from.to_string()))
        .filter(strategy_daily_stats::date.le(to.to_string()))
        .load(&mut conn)
        .unwrap_or_default();

    Ok(rows)
}

pub fn load_open_positions(db_path: &Path) -> Result<i64> {
    let pool = connect(db_path)?;
    let mut conn = pool
        .get()
        .map_err(|e| Error::Config(ConfigError::Other(e.to_string())))?;

    let open_count: i64 = trades::table
        .filter(trades::status.eq("open"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    Ok(open_count)
}

pub fn date_range_today() -> (NaiveDate, NaiveDate, String) {
    let today = Utc::now().date_naive();
    (today, today, "Today".to_string())
}

pub fn date_range_week() -> (NaiveDate, NaiveDate, String) {
    let today = Utc::now().date_naive();
    let week_ago = today - Duration::days(7);
    (week_ago, today, "Last 7 Days".to_string())
}

pub fn date_range_history(days: u32) -> (NaiveDate, NaiveDate, String) {
    let today = Utc::now().date_naive();
    let start = today - Duration::days(i64::from(days));
    (start, today, format!("Last {days} Days"))
}

pub fn load_daily_rows(
    db_path: &Path,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<DailyStatsRow>> {
    let pool = connect(db_path)?;
    let mut conn = pool
        .get()
        .map_err(|e| Error::Config(ConfigError::Other(e.to_string())))?;

    let rows: Vec<DailyStatsRow> = daily_stats::table
        .filter(daily_stats::date.ge(from.to_string()))
        .filter(daily_stats::date.le(to.to_string()))
        .order(daily_stats::date.desc())
        .load(&mut conn)
        .unwrap_or_default();

    Ok(rows)
}

pub fn export_daily_csv(db_path: &Path, from: NaiveDate, to: NaiveDate) -> Result<String> {
    let pool = connect(db_path)?;
    Ok(crate::adapters::statistics::export_daily_csv(
        &pool, from, to,
    ))
}

pub fn prune_old_records(db_path: &Path, retention_days: u32) -> Result<()> {
    let pool = connect(db_path)?;
    let recorder = StatsRecorder::new(pool);
    recorder.prune_old_records(retention_days);
    Ok(())
}
