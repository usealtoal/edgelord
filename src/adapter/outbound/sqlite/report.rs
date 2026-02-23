//! SQLite read-side report adapter.
//!
//! Provides implementations of the status and statistics report reader
//! traits for CLI commands and status displays.

use chrono::{Duration, NaiveDate, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};

use crate::adapter::outbound::sqlite::database::model::{
    DailyStatsRow, OpportunityRow, StrategyDailyStatsRow, TradeRow,
};
use crate::adapter::outbound::sqlite::database::schema::{
    daily_stats, opportunities, strategy_daily_stats, trades,
};
use crate::adapter::outbound::sqlite::recorder::{
    export_daily_csv as export_csv_impl, f32_to_decimal, SqliteRecorder,
};
use crate::domain::stats::StatsSummary;
use crate::error::{ConfigError, Error, Result};
use crate::port::outbound::report::{
    DailyStatsRecord, RecentActivity, StatisticsReportReader, StatusReportReader, StatusSnapshot,
    StrategyDailyStatsRecord,
};

/// SQLite report reader for status and statistics queries.
///
/// Implements both [`StatusReportReader`] and [`StatisticsReportReader`]
/// traits for CLI status and statistics commands.
pub struct SqliteReportReader {
    /// SQLite database URL or file path.
    database_url: String,
}

impl SqliteReportReader {
    /// Create a report reader backed by the given SQLite database URL.
    #[must_use]
    pub fn new(database_url: impl Into<String>) -> Self {
        Self {
            database_url: database_url.into(),
        }
    }

    /// Establish a connection pool to the database.
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
        let recorder = SqliteRecorder::new(pool);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::outbound::sqlite::database::model::{
        DailyStatsRow, NewOpportunityRow, NewTradeRow, StrategyDailyStatsRow,
    };
    use crate::adapter::outbound::sqlite::database::schema::{
        daily_stats, opportunities, strategy_daily_stats, trades,
    };
    use diesel::r2d2::Pool;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    /// Creates a unique test database pool with migrations run.
    /// Returns both the database URL and the pool (which must be kept alive).
    fn setup_test_db() -> (String, Pool<ConnectionManager<SqliteConnection>>) {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let db_url = format!("file:report_test_db_{}?mode=memory&cache=shared", id);

        let manager = ConnectionManager::<SqliteConnection>::new(&db_url);
        let pool = Pool::builder().max_size(5).build(manager).unwrap();
        let mut conn = pool.get().expect("Failed to get connection");
        conn.run_pending_migrations(MIGRATIONS)
            .expect("Failed to run migrations");

        (db_url, pool)
    }

    // -------------------------------------------------------------------------
    // StatusReportReader tests
    // -------------------------------------------------------------------------

    #[test]
    fn load_status_returns_empty_snapshot_for_empty_db() {
        let (db_url, _pool) = setup_test_db();

        let reader = SqliteReportReader::new(&db_url);
        let snapshot = reader.load_status().unwrap();

        assert!(snapshot.today.is_none());
        assert!(snapshot.week_rows.is_empty());
        assert_eq!(snapshot.open_positions, 0);
        assert_eq!(snapshot.distinct_markets, 0);
        assert!((snapshot.current_exposure - 0.0).abs() < 0.01);
        assert!(snapshot.recent_activity.is_empty());
    }

    #[test]
    fn load_status_returns_today_stats() {
        let (db_url, pool) = setup_test_db();

        // Insert today's stats
        {
            let mut conn = pool.get().unwrap();
            let today = Utc::now().date_naive().to_string();
            let row = DailyStatsRow {
                date: today,
                opportunities_detected: 100,
                opportunities_executed: 50,
                trades_opened: 25,
                profit_realized: 500.0,
                ..Default::default()
            };
            diesel::insert_into(daily_stats::table)
                .values(&row)
                .execute(&mut conn)
                .unwrap();
        }

        let reader = SqliteReportReader::new(&db_url);
        let snapshot = reader.load_status().unwrap();

        assert!(snapshot.today.is_some());
        let today_record = snapshot.today.unwrap();
        assert_eq!(today_record.opportunities_detected, 100);
        assert_eq!(today_record.opportunities_executed, 50);
    }

    #[test]
    fn load_status_returns_week_rows() {
        let (db_url, pool) = setup_test_db();
        {
            let mut conn = pool.get().unwrap();

            // Insert 7 days of stats
            let today = Utc::now().date_naive();
            for i in 0..7 {
                let date = (today - Duration::days(i)).to_string();
                let row = DailyStatsRow {
                    date,
                    opportunities_detected: 10 + i as i32,
                    ..Default::default()
                };
                diesel::insert_into(daily_stats::table)
                    .values(&row)
                    .execute(&mut conn)
                    .unwrap();
            }
        }

        let reader = SqliteReportReader::new(&db_url);
        let snapshot = reader.load_status().unwrap();

        assert_eq!(snapshot.week_rows.len(), 7);
    }

    #[test]
    fn load_status_counts_open_positions() {
        let (db_url, pool) = setup_test_db();
        {
            let mut conn = pool.get().unwrap();

            // Insert an opportunity first
            let opp = NewOpportunityRow {
                strategy: "test".to_string(),
                market_ids: "[\"m1\"]".to_string(),
                edge: 0.05,
                expected_profit: 5.0,
                detected_at: Utc::now().to_rfc3339(),
                executed: 1,
                rejected_reason: None,
            };
            diesel::insert_into(opportunities::table)
                .values(&opp)
                .execute(&mut conn)
                .unwrap();

            // Insert open trades
            for i in 0..3 {
                let trade = NewTradeRow {
                    opportunity_id: 1,
                    strategy: "test".to_string(),
                    market_ids: format!("[\"market-{}\"]", i),
                    legs: "[]".to_string(),
                    size: 100.0,
                    expected_profit: 5.0,
                    status: "open".to_string(),
                    opened_at: Utc::now().to_rfc3339(),
                };
                diesel::insert_into(trades::table)
                    .values(&trade)
                    .execute(&mut conn)
                    .unwrap();
            }
        }

        let reader = SqliteReportReader::new(&db_url);
        let snapshot = reader.load_status().unwrap();

        assert_eq!(snapshot.open_positions, 3);
    }

    #[test]
    fn load_status_calculates_current_exposure() {
        let (db_url, pool) = setup_test_db();
        {
            let mut conn = pool.get().unwrap();

            // Insert opportunity
            let opp = NewOpportunityRow {
                strategy: "test".to_string(),
                market_ids: "[\"m1\"]".to_string(),
                edge: 0.05,
                expected_profit: 5.0,
                detected_at: Utc::now().to_rfc3339(),
                executed: 1,
                rejected_reason: None,
            };
            diesel::insert_into(opportunities::table)
                .values(&opp)
                .execute(&mut conn)
                .unwrap();

            // Insert open trades with different sizes
            for size in [100.0, 250.0, 150.0] {
                let trade = NewTradeRow {
                    opportunity_id: 1,
                    strategy: "test".to_string(),
                    market_ids: "[\"m1\"]".to_string(),
                    legs: "[]".to_string(),
                    size,
                    expected_profit: 5.0,
                    status: "open".to_string(),
                    opened_at: Utc::now().to_rfc3339(),
                };
                diesel::insert_into(trades::table)
                    .values(&trade)
                    .execute(&mut conn)
                    .unwrap();
            }
        }

        let reader = SqliteReportReader::new(&db_url);
        let snapshot = reader.load_status().unwrap();

        assert!((snapshot.current_exposure - 500.0).abs() < 0.01);
    }

    #[test]
    fn load_status_returns_recent_activity() {
        let (db_url, pool) = setup_test_db();
        {
            let mut conn = pool.get().unwrap();

            // Insert opportunity
            let opp = NewOpportunityRow {
                strategy: "test".to_string(),
                market_ids: "[\"m1\"]".to_string(),
                edge: 0.05,
                expected_profit: 5.0,
                detected_at: Utc::now().to_rfc3339(),
                executed: 1,
                rejected_reason: None,
            };
            diesel::insert_into(opportunities::table)
                .values(&opp)
                .execute(&mut conn)
                .unwrap();

            // Insert closed trade
            let trade = NewTradeRow {
                opportunity_id: 1,
                strategy: "test".to_string(),
                market_ids: "[\"m1\"]".to_string(),
                legs: "[]".to_string(),
                size: 100.0,
                expected_profit: 5.0,
                status: "closed".to_string(),
                opened_at: Utc::now().to_rfc3339(),
            };
            diesel::insert_into(trades::table)
                .values(&trade)
                .execute(&mut conn)
                .unwrap();

            // Update with closed info
            diesel::update(trades::table.filter(trades::id.eq(1)))
                .set((
                    trades::closed_at.eq(Some(Utc::now().to_rfc3339())),
                    trades::realized_profit.eq(Some(10.0f32)),
                ))
                .execute(&mut conn)
                .unwrap();

            // Insert rejected opportunity
            let rejected_opp = NewOpportunityRow {
                strategy: "test".to_string(),
                market_ids: "[\"m2\"]".to_string(),
                edge: 0.03,
                expected_profit: 3.0,
                detected_at: Utc::now().to_rfc3339(),
                executed: 0,
                rejected_reason: Some("risk_limit".to_string()),
            };
            diesel::insert_into(opportunities::table)
                .values(&rejected_opp)
                .execute(&mut conn)
                .unwrap();
        }

        let reader = SqliteReportReader::new(&db_url);
        let snapshot = reader.load_status().unwrap();

        assert!(!snapshot.recent_activity.is_empty());
    }

    // -------------------------------------------------------------------------
    // StatisticsReportReader tests
    // -------------------------------------------------------------------------

    #[test]
    fn load_summary_aggregates_date_range() {
        let (db_url, pool) = setup_test_db();
        {
            let mut conn = pool.get().unwrap();

            // Insert multiple days
            for (date, opp_count) in [("2026-01-01", 10), ("2026-01-02", 20), ("2026-01-03", 30)] {
                let row = DailyStatsRow {
                    date: date.to_string(),
                    opportunities_detected: opp_count,
                    ..Default::default()
                };
                diesel::insert_into(daily_stats::table)
                    .values(&row)
                    .execute(&mut conn)
                    .unwrap();
            }
        }

        let reader = SqliteReportReader::new(&db_url);
        let from = NaiveDate::parse_from_str("2026-01-01", "%Y-%m-%d").unwrap();
        let to = NaiveDate::parse_from_str("2026-01-03", "%Y-%m-%d").unwrap();

        let summary = reader.load_summary(from, to).unwrap();

        assert_eq!(summary.opportunities_detected, 60);
    }

    #[test]
    fn load_strategy_breakdown_returns_per_strategy_records() {
        let (db_url, pool) = setup_test_db();
        {
            let mut conn = pool.get().unwrap();

            let rows = vec![
                StrategyDailyStatsRow {
                    date: "2026-01-01".to_string(),
                    strategy: "strat_a".to_string(),
                    opportunities_detected: 50,
                    ..Default::default()
                },
                StrategyDailyStatsRow {
                    date: "2026-01-01".to_string(),
                    strategy: "strat_b".to_string(),
                    opportunities_detected: 30,
                    ..Default::default()
                },
            ];

            for row in rows {
                diesel::insert_into(strategy_daily_stats::table)
                    .values(&row)
                    .execute(&mut conn)
                    .unwrap();
            }
        }

        let reader = SqliteReportReader::new(&db_url);
        let from = NaiveDate::parse_from_str("2026-01-01", "%Y-%m-%d").unwrap();
        let to = NaiveDate::parse_from_str("2026-01-01", "%Y-%m-%d").unwrap();

        let breakdown = reader.load_strategy_breakdown(from, to).unwrap();

        assert_eq!(breakdown.len(), 2);
        assert!(breakdown.iter().any(|r| r.strategy == "strat_a"));
        assert!(breakdown.iter().any(|r| r.strategy == "strat_b"));
    }

    #[test]
    fn load_open_positions_returns_count() {
        let (db_url, pool) = setup_test_db();
        {
            let mut conn = pool.get().unwrap();

            // Insert opportunity
            let opp = NewOpportunityRow {
                strategy: "test".to_string(),
                market_ids: "[\"m1\"]".to_string(),
                edge: 0.05,
                expected_profit: 5.0,
                detected_at: Utc::now().to_rfc3339(),
                executed: 1,
                rejected_reason: None,
            };
            diesel::insert_into(opportunities::table)
                .values(&opp)
                .execute(&mut conn)
                .unwrap();

            // Insert trades with mixed status
            for (status, _i) in [("open", 0), ("open", 1), ("closed", 2)] {
                let trade = NewTradeRow {
                    opportunity_id: 1,
                    strategy: "test".to_string(),
                    market_ids: "[\"m1\"]".to_string(),
                    legs: "[]".to_string(),
                    size: 100.0,
                    expected_profit: 5.0,
                    status: status.to_string(),
                    opened_at: Utc::now().to_rfc3339(),
                };
                diesel::insert_into(trades::table)
                    .values(&trade)
                    .execute(&mut conn)
                    .unwrap();
            }
        }

        let reader = SqliteReportReader::new(&db_url);
        let count = reader.load_open_positions().unwrap();

        assert_eq!(count, 2);
    }

    #[test]
    fn load_daily_rows_returns_ordered_records() {
        let (db_url, pool) = setup_test_db();
        {
            let mut conn = pool.get().unwrap();

            for date in ["2026-01-03", "2026-01-01", "2026-01-02"] {
                let row = DailyStatsRow {
                    date: date.to_string(),
                    ..Default::default()
                };
                diesel::insert_into(daily_stats::table)
                    .values(&row)
                    .execute(&mut conn)
                    .unwrap();
            }
        }

        let reader = SqliteReportReader::new(&db_url);
        let from = NaiveDate::parse_from_str("2026-01-01", "%Y-%m-%d").unwrap();
        let to = NaiveDate::parse_from_str("2026-01-03", "%Y-%m-%d").unwrap();

        let rows = reader.load_daily_rows(from, to).unwrap();

        assert_eq!(rows.len(), 3);
        // Should be ordered descending
        assert_eq!(rows[0].date, "2026-01-03");
        assert_eq!(rows[1].date, "2026-01-02");
        assert_eq!(rows[2].date, "2026-01-01");
    }

    #[test]
    fn export_daily_csv_returns_valid_csv() {
        let (db_url, pool) = setup_test_db();
        {
            let mut conn = pool.get().unwrap();

            let row = DailyStatsRow {
                date: "2026-01-15".to_string(),
                opportunities_detected: 100,
                opportunities_executed: 50,
                ..Default::default()
            };
            diesel::insert_into(daily_stats::table)
                .values(&row)
                .execute(&mut conn)
                .unwrap();
        }

        let reader = SqliteReportReader::new(&db_url);
        let from = NaiveDate::parse_from_str("2026-01-01", "%Y-%m-%d").unwrap();
        let to = NaiveDate::parse_from_str("2026-01-31", "%Y-%m-%d").unwrap();

        let csv = reader.export_daily_csv(from, to).unwrap();

        assert!(csv.contains("date,"));
        assert!(csv.contains("2026-01-15"));
    }

    #[test]
    fn prune_old_records_removes_old_data() {
        let (db_url, pool) = setup_test_db();
        {
            let mut conn = pool.get().unwrap();

            // Insert old opportunity
            let old_opp = NewOpportunityRow {
                strategy: "test".to_string(),
                market_ids: "[]".to_string(),
                edge: 0.05,
                expected_profit: 5.0,
                detected_at: "2020-01-01T00:00:00Z".to_string(),
                executed: 1,
                rejected_reason: None,
            };
            diesel::insert_into(opportunities::table)
                .values(&old_opp)
                .execute(&mut conn)
                .unwrap();

            // Insert recent opportunity
            let recent_opp = NewOpportunityRow {
                strategy: "test".to_string(),
                market_ids: "[]".to_string(),
                edge: 0.05,
                expected_profit: 5.0,
                detected_at: Utc::now().to_rfc3339(),
                executed: 1,
                rejected_reason: None,
            };
            diesel::insert_into(opportunities::table)
                .values(&recent_opp)
                .execute(&mut conn)
                .unwrap();
        }

        let reader = SqliteReportReader::new(&db_url);
        reader.prune_old_records(30).unwrap();

        {
            let mut conn = pool.get().unwrap();
            let count: i64 = opportunities::table.count().get_result(&mut conn).unwrap();
            assert_eq!(count, 1);
        }
    }

    // -------------------------------------------------------------------------
    // Helper function tests
    // -------------------------------------------------------------------------

    #[test]
    fn extract_time_from_rfc3339() {
        let timestamp = "2026-01-15T14:30:45Z";
        let time = extract_time(timestamp);
        assert_eq!(time, "14:30:45");
    }

    #[test]
    fn extract_time_from_space_separated() {
        let timestamp = "2026-01-15 14:30:45";
        let time = extract_time(timestamp);
        assert_eq!(time, "14:30:45");
    }

    #[test]
    fn extract_time_returns_original_if_no_separator() {
        let timestamp = "14:30:45";
        let time = extract_time(timestamp);
        assert_eq!(time, "14:30:45");
    }

    #[test]
    fn extract_market_description_single_short_id() {
        let json = r#"["short-id"]"#;
        let desc = extract_market_description(json);
        assert_eq!(desc, "short-id");
    }

    #[test]
    fn extract_market_description_single_long_id() {
        let json = r#"["this-is-a-very-long-market-id"]"#;
        let desc = extract_market_description(json);
        assert_eq!(desc, "this-is-a-ve...");
    }

    #[test]
    fn extract_market_description_multiple_markets() {
        let json = r#"["m1", "m2", "m3"]"#;
        let desc = extract_market_description(json);
        assert_eq!(desc, "3 markets");
    }

    #[test]
    fn extract_market_description_empty_array() {
        let json = r#"[]"#;
        let desc = extract_market_description(json);
        assert_eq!(desc, "unknown market");
    }

    #[test]
    fn extract_market_description_invalid_json() {
        let json = "not valid json";
        let desc = extract_market_description(json);
        assert_eq!(desc, "unknown market");
    }

    // -------------------------------------------------------------------------
    // From implementations
    // -------------------------------------------------------------------------

    #[test]
    fn daily_stats_record_from_row() {
        let row = DailyStatsRow {
            date: "2026-01-15".to_string(),
            opportunities_detected: 100,
            opportunities_executed: 50,
            opportunities_rejected: 10,
            trades_opened: 25,
            trades_closed: 20,
            profit_realized: 500.0,
            loss_realized: 100.0,
            win_count: 15,
            loss_count: 5,
            total_volume: 10000.0,
            peak_exposure: 2000.0,
            latency_sum_ms: 5000,
            latency_count: 100,
        };

        let record = DailyStatsRecord::from(row);

        assert_eq!(record.date, "2026-01-15");
        assert_eq!(record.opportunities_detected, 100);
        assert_eq!(record.profit_realized, 500.0);
    }

    #[test]
    fn strategy_daily_stats_record_from_row() {
        let row = StrategyDailyStatsRow {
            date: "2026-01-15".to_string(),
            strategy: "single_condition".to_string(),
            opportunities_detected: 50,
            opportunities_executed: 25,
            trades_opened: 20,
            trades_closed: 18,
            profit_realized: 250.0,
            win_count: 12,
            loss_count: 6,
        };

        let record = StrategyDailyStatsRecord::from(row);

        assert_eq!(record.date, "2026-01-15");
        assert_eq!(record.strategy, "single_condition");
        assert_eq!(record.opportunities_detected, 50);
    }

    // -------------------------------------------------------------------------
    // Edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn load_summary_empty_date_range() {
        let (db_url, _pool) = setup_test_db();

        let reader = SqliteReportReader::new(&db_url);
        let from = NaiveDate::parse_from_str("2030-01-01", "%Y-%m-%d").unwrap();
        let to = NaiveDate::parse_from_str("2030-12-31", "%Y-%m-%d").unwrap();

        let summary = reader.load_summary(from, to).unwrap();

        assert_eq!(summary.opportunities_detected, 0);
        assert_eq!(summary.trades_opened, 0);
    }

    #[test]
    fn load_status_counts_distinct_markets() {
        let (db_url, pool) = setup_test_db();
        {
            let mut conn = pool.get().unwrap();

            // Insert opportunity
            let opp = NewOpportunityRow {
                strategy: "test".to_string(),
                market_ids: "[\"m1\"]".to_string(),
                edge: 0.05,
                expected_profit: 5.0,
                detected_at: Utc::now().to_rfc3339(),
                executed: 1,
                rejected_reason: None,
            };
            diesel::insert_into(opportunities::table)
                .values(&opp)
                .execute(&mut conn)
                .unwrap();

            // Insert trades with overlapping markets
            let trades_data = [
                r#"["market-a", "market-b"]"#,
                r#"["market-b", "market-c"]"#,
                r#"["market-a"]"#,
            ];

            for market_ids in trades_data {
                let trade = NewTradeRow {
                    opportunity_id: 1,
                    strategy: "test".to_string(),
                    market_ids: market_ids.to_string(),
                    legs: "[]".to_string(),
                    size: 100.0,
                    expected_profit: 5.0,
                    status: "open".to_string(),
                    opened_at: Utc::now().to_rfc3339(),
                };
                diesel::insert_into(trades::table)
                    .values(&trade)
                    .execute(&mut conn)
                    .unwrap();
            }
        }

        let reader = SqliteReportReader::new(&db_url);
        let snapshot = reader.load_status().unwrap();

        // Should have 3 distinct markets: a, b, c
        assert_eq!(snapshot.distinct_markets, 3);
    }
}
