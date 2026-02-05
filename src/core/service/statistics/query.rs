//! Read-only statistics queries.

use chrono::{NaiveDate, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use tracing::warn;

use super::export::summary_from_rows;
use super::types::StatsSummary;
use crate::core::db::model::DailyStatsRow;
use crate::core::db::schema::daily_stats;

pub(crate) fn summary_for_range(
    pool: &Pool<ConnectionManager<SqliteConnection>>,
    from: NaiveDate,
    to: NaiveDate,
) -> StatsSummary {
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to get DB connection for stats query: {e}");
            return StatsSummary::default();
        }
    };

    let rows: Vec<DailyStatsRow> = daily_stats::table
        .filter(daily_stats::date.ge(from.to_string()))
        .filter(daily_stats::date.le(to.to_string()))
        .load(&mut conn)
        .unwrap_or_default();

    summary_from_rows(&rows)
}

pub(crate) fn summary_for_today(pool: &Pool<ConnectionManager<SqliteConnection>>) -> StatsSummary {
    let today = Utc::now().date_naive();
    summary_for_range(pool, today, today)
}
