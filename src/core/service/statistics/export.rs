//! CSV export and recent opportunity queries.

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use tracing::warn;

use super::convert::f32_to_decimal;
use super::types::{OpportunitySummary, StatsSummary};
use crate::core::db::schema::{daily_stats, opportunities};
use crate::core::db::model::{DailyStatsRow, OpportunityRow};

/// Export daily stats to CSV format.
pub fn export_daily_csv(
    pool: &Pool<ConnectionManager<SqliteConnection>>,
    from: chrono::NaiveDate,
    to: chrono::NaiveDate,
) -> String {
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to get DB connection for CSV export: {e}");
            return String::new();
        }
    };

    let rows: Vec<DailyStatsRow> = daily_stats::table
        .filter(daily_stats::date.ge(from.to_string()))
        .filter(daily_stats::date.le(to.to_string()))
        .order(daily_stats::date.asc())
        .load(&mut conn)
        .unwrap_or_default();

    let mut csv = String::from(
        "date,opportunities_detected,opportunities_executed,trades_opened,trades_closed,profit,loss,net,win_count,loss_count,win_rate,volume,peak_exposure\n"
    );

    for row in rows {
        let net = row.profit_realized - row.loss_realized;
        let total = row.win_count + row.loss_count;
        let win_rate = if total > 0 {
            row.win_count as f32 / total as f32 * 100.0
        } else {
            0.0
        };
        csv.push_str(&format!(
            "{},{},{},{},{},{:.2},{:.2},{:.2},{},{},{:.1},{:.2},{:.2}\n",
            row.date,
            row.opportunities_detected,
            row.opportunities_executed,
            row.trades_opened,
            row.trades_closed,
            row.profit_realized,
            row.loss_realized,
            net,
            row.win_count,
            row.loss_count,
            win_rate,
            row.total_volume,
            row.peak_exposure
        ));
    }

    csv
}

/// Get recent opportunities.
pub fn recent_opportunities(
    pool: &Pool<ConnectionManager<SqliteConnection>>,
    limit: i64,
) -> Vec<OpportunitySummary> {
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to get DB connection for opportunities query: {e}");
            return Vec::new();
        }
    };

    let rows: Vec<OpportunityRow> = opportunities::table
        .order(opportunities::detected_at.desc())
        .limit(limit)
        .load(&mut conn)
        .unwrap_or_default();

    rows.into_iter()
        .map(|r| OpportunitySummary {
            id: r.id.unwrap_or(0),
            strategy: r.strategy,
            edge: f32_to_decimal(r.edge),
            expected_profit: f32_to_decimal(r.expected_profit),
            executed: r.executed != 0,
            rejected_reason: r.rejected_reason,
            detected_at: r.detected_at,
        })
        .collect()
}

pub fn summary_from_rows(rows: &[DailyStatsRow]) -> StatsSummary {
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
    summary
}
