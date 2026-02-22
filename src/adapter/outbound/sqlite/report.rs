//! SQLite read-side report adapter.

use chrono::{Duration, NaiveDate, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};

use crate::adapter::outbound::sqlite::database::model::{
    DailyStatsRow, OpportunityRow, StrategyDailyStatsRow, TradeRow,
};
use crate::adapter::outbound::sqlite::database::schema::{
    daily_stats, opportunities, strategy_daily_stats, trades,
};
use crate::adapter::outbound::sqlite::stats_recorder::{
    export_daily_csv as export_csv_impl, f32_to_decimal, SqliteStatsRecorder,
};
use crate::domain::stats::StatsSummary;
use crate::error::{ConfigError, Error, Result};
use crate::port::outbound::report::{
    DailyStatsRecord, RecentActivity, StatisticsReportReader, StatusReportReader, StatusSnapshot,
    StrategyDailyStatsRecord,
};

/// SQLite report reader that provides status and statistics query surfaces.
pub struct SqliteReportReader {
    database_url: String,
}

impl SqliteReportReader {
    /// Creates a report reader backed by a sqlite URL.
    #[must_use]
    pub fn new(database_url: impl Into<String>) -> Self {
        Self {
            database_url: database_url.into(),
        }
    }

    fn connect(&self) -> Result<Pool<ConnectionManager<SqliteConnection>>> {
        let manager = ConnectionManager::<SqliteConnection>::new(&self.database_url);
        Pool::builder()
            .max_size(1)
            .build(manager)
            .map_err(|error| Error::Config(ConfigError::Other(error.to_string())))
    }
}

impl StatusReportReader for SqliteReportReader {
    fn load_status(&self) -> Result<StatusSnapshot> {
        let pool = self.connect()?;
        let mut conn = pool
            .get()
            .map_err(|error| Error::Config(ConfigError::Other(error.to_string())))?;

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

        let open_trades: Vec<TradeRow> = trades::table
            .filter(trades::status.eq("open"))
            .load(&mut conn)
            .unwrap_or_default();

        let open_positions = open_trades.len() as i64;
        let distinct_markets = open_trades
            .iter()
            .flat_map(|trade| {
                serde_json::from_str::<Vec<String>>(&trade.market_ids).unwrap_or_default()
            })
            .collect::<std::collections::HashSet<_>>()
            .len() as i64;
        let current_exposure: f32 = open_trades.iter().map(|trade| trade.size).sum();

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

        let mut recent_activity: Vec<RecentActivity> = Vec::new();

        for trade in recent_trades {
            if let Some(closed_at) = &trade.closed_at {
                recent_activity.push(RecentActivity::Executed {
                    timestamp: extract_time(closed_at),
                    profit: trade.realized_profit.unwrap_or(0.0),
                    market_description: extract_market_description(&trade.market_ids),
                });
            }
        }

        for opportunity in recent_rejected {
            recent_activity.push(RecentActivity::Rejected {
                timestamp: extract_time(&opportunity.detected_at),
                reason: opportunity
                    .rejected_reason
                    .unwrap_or_else(|| "unknown".to_string()),
            });
        }

        recent_activity.sort_by(|left, right| {
            let left_ts = match left {
                RecentActivity::Executed { timestamp, .. } => timestamp,
                RecentActivity::Rejected { timestamp, .. } => timestamp,
            };
            let right_ts = match right {
                RecentActivity::Executed { timestamp, .. } => timestamp,
                RecentActivity::Rejected { timestamp, .. } => timestamp,
            };
            right_ts.cmp(left_ts)
        });
        recent_activity.truncate(5);

        Ok(StatusSnapshot {
            today: today_row.map(DailyStatsRecord::from),
            week_rows: week_rows.into_iter().map(DailyStatsRecord::from).collect(),
            open_positions,
            distinct_markets,
            current_exposure,
            recent_activity,
        })
    }
}

impl StatisticsReportReader for SqliteReportReader {
    fn load_summary(&self, from: NaiveDate, to: NaiveDate) -> Result<StatsSummary> {
        let pool = self.connect()?;
        let mut conn = pool
            .get()
            .map_err(|error| Error::Config(ConfigError::Other(error.to_string())))?;

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

    fn load_strategy_breakdown(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<StrategyDailyStatsRecord>> {
        let pool = self.connect()?;
        let mut conn = pool
            .get()
            .map_err(|error| Error::Config(ConfigError::Other(error.to_string())))?;

        let rows: Vec<StrategyDailyStatsRow> = strategy_daily_stats::table
            .filter(strategy_daily_stats::date.ge(from.to_string()))
            .filter(strategy_daily_stats::date.le(to.to_string()))
            .load(&mut conn)
            .unwrap_or_default();

        Ok(rows
            .into_iter()
            .map(StrategyDailyStatsRecord::from)
            .collect())
    }

    fn load_open_positions(&self) -> Result<i64> {
        let pool = self.connect()?;
        let mut conn = pool
            .get()
            .map_err(|error| Error::Config(ConfigError::Other(error.to_string())))?;

        let open_count: i64 = trades::table
            .filter(trades::status.eq("open"))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0);

        Ok(open_count)
    }

    fn load_daily_rows(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<DailyStatsRecord>> {
        let pool = self.connect()?;
        let mut conn = pool
            .get()
            .map_err(|error| Error::Config(ConfigError::Other(error.to_string())))?;

        let rows: Vec<DailyStatsRow> = daily_stats::table
            .filter(daily_stats::date.ge(from.to_string()))
            .filter(daily_stats::date.le(to.to_string()))
            .order(daily_stats::date.desc())
            .load(&mut conn)
            .unwrap_or_default();

        Ok(rows.into_iter().map(DailyStatsRecord::from).collect())
    }

    fn export_daily_csv(&self, from: NaiveDate, to: NaiveDate) -> Result<String> {
        let pool = self.connect()?;
        Ok(export_csv_impl(&pool, from, to))
    }

    fn prune_old_records(&self, retention_days: u32) -> Result<()> {
        let pool = self.connect()?;
        let recorder = SqliteStatsRecorder::new(pool);
        recorder.prune_old_records(retention_days);
        Ok(())
    }
}

impl From<DailyStatsRow> for DailyStatsRecord {
    fn from(row: DailyStatsRow) -> Self {
        Self {
            date: row.date,
            opportunities_detected: row.opportunities_detected,
            opportunities_executed: row.opportunities_executed,
            opportunities_rejected: row.opportunities_rejected,
            trades_opened: row.trades_opened,
            trades_closed: row.trades_closed,
            profit_realized: row.profit_realized,
            loss_realized: row.loss_realized,
            win_count: row.win_count,
            loss_count: row.loss_count,
            total_volume: row.total_volume,
            peak_exposure: row.peak_exposure,
            latency_sum_ms: row.latency_sum_ms,
            latency_count: row.latency_count,
        }
    }
}

impl From<StrategyDailyStatsRow> for StrategyDailyStatsRecord {
    fn from(row: StrategyDailyStatsRow) -> Self {
        Self {
            date: row.date,
            strategy: row.strategy,
            opportunities_detected: row.opportunities_detected,
            opportunities_executed: row.opportunities_executed,
            trades_opened: row.trades_opened,
            trades_closed: row.trades_closed,
            profit_realized: row.profit_realized,
            win_count: row.win_count,
            loss_count: row.loss_count,
        }
    }
}

fn extract_time(timestamp: &str) -> String {
    if let Some(t_pos) = timestamp.find('T') {
        timestamp[t_pos + 1..].chars().take(8).collect()
    } else if let Some(space_pos) = timestamp.find(' ') {
        timestamp[space_pos + 1..].chars().take(8).collect()
    } else {
        timestamp.to_string()
    }
}

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
