//! SQLite statistics persistence.
//!
//! Records opportunities, trades, and daily aggregates for historical analysis.
//! Implements the [`StatsRecorder`](crate::port::outbound::stats::StatsRecorder)
//! trait for the statistics recording port.

use std::sync::Arc;

use chrono::{NaiveDate, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::OptionalExtension;
use diesel::SqliteConnection;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;
use tracing::{debug, warn};

use crate::adapter::outbound::sqlite::database::connection::configure_sqlite_connection;
use crate::adapter::outbound::sqlite::database::model::{
    DailyStatsRow, NewOpportunityRow, NewTradeRow, OpportunityRow, StrategyDailyStatsRow, TradeRow,
};
use crate::adapter::outbound::sqlite::database::schema::{
    daily_stats, opportunities, strategy_daily_stats, trades,
};
use crate::domain::stats::{
    OpportunitySummary, RecordedOpportunity, StatsSummary, TradeCloseEvent, TradeOpenEvent,
};

/// Convert a decimal to f32 for storage.
fn decimal_to_f32(d: Decimal) -> f32 {
    d.to_f32().unwrap_or(0.0)
}

/// Convert f32 to Decimal for summary calculations.
#[must_use]
pub fn f32_to_decimal(f: f32) -> Decimal {
    Decimal::from_f32(f).unwrap_or(Decimal::ZERO)
}

/// SQLite-backed statistics recorder.
///
/// Records trading events and maintains daily aggregate statistics.
/// Implements the [`StatsRecorder`](crate::port::outbound::stats::StatsRecorder) trait.
pub struct SqliteRecorder {
    /// Database connection pool.
    pool: Pool<ConnectionManager<SqliteConnection>>,
}

#[derive(QueryableByName)]
struct LastInsertRowId {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    #[diesel(column_name = "id")]
    id: i32,
}

impl SqliteRecorder {
    /// Create a new statistics recorder with the given connection pool.
    #[must_use]
    pub fn new(pool: Pool<ConnectionManager<SqliteConnection>>) -> Self {
        Self { pool }
    }

    /// Record an opportunity detection event.
    ///
    /// Returns the database row ID if successful, or `None` on failure.
    pub fn record_opportunity(&self, event: &RecordedOpportunity) -> Option<i32> {
        let market_ids_json = serde_json::to_string(&event.market_ids).unwrap_or_default();
        let now = Utc::now().to_rfc3339();
        let today = Utc::now().date_naive().to_string();

        let row = NewOpportunityRow {
            strategy: event.strategy.clone(),
            market_ids: market_ids_json,
            edge: decimal_to_f32(event.edge),
            expected_profit: decimal_to_f32(event.expected_profit),
            detected_at: now,
            executed: i32::from(event.executed),
            rejected_reason: event.rejected_reason.clone(),
        };

        let mut conn = self.pool.get().ok()?;
        if let Err(e) = configure_sqlite_connection(&mut conn) {
            warn!(error = %e, "Failed to configure SQLite connection");
        }

        let id = conn.transaction(|conn| {
            diesel::insert_into(opportunities::table)
                .values(&row)
                .execute(conn)?;

            let id: i32 = diesel::sql_query("SELECT last_insert_rowid() AS id")
                .get_result::<LastInsertRowId>(conn)
                .map(|row| row.id)?;

            self.update_daily_stats_with_conn(conn, &today, &event.strategy, |daily, strategy| {
                daily.opportunities_detected += 1;
                strategy.opportunities_detected += 1;
                if event.executed {
                    daily.opportunities_executed += 1;
                    strategy.opportunities_executed += 1;
                } else if event.rejected_reason.is_some() {
                    daily.opportunities_rejected += 1;
                }
            })?;

            Ok::<i32, diesel::result::Error>(id)
        });

        match id {
            Ok(id) => {
                debug!(id = id, strategy = %event.strategy, "Recorded opportunity");
                Some(id)
            }
            Err(e) => {
                warn!(error = %e, "Opportunity transaction failed");
                None
            }
        }
    }

    /// Record a trade opening event.
    ///
    /// Returns the database row ID if successful, or `None` on failure.
    pub fn record_trade_open(&self, event: &TradeOpenEvent) -> Option<i32> {
        let market_ids_json = serde_json::to_string(&event.market_ids).unwrap_or_default();
        let legs_json = serde_json::to_string(&event.legs).unwrap_or_default();
        let now = Utc::now().to_rfc3339();
        let today = Utc::now().date_naive().to_string();

        let row = NewTradeRow {
            opportunity_id: event.opportunity_id,
            strategy: event.strategy.clone(),
            market_ids: market_ids_json,
            legs: legs_json,
            size: decimal_to_f32(event.size),
            expected_profit: decimal_to_f32(event.expected_profit),
            status: "open".to_string(),
            opened_at: now,
        };

        let mut conn = self.pool.get().ok()?;
        if let Err(e) = configure_sqlite_connection(&mut conn) {
            warn!(error = %e, "Failed to configure SQLite connection");
        }

        let id = conn.transaction(|conn| {
            diesel::insert_into(trades::table)
                .values(&row)
                .execute(conn)?;

            let id: i32 = diesel::sql_query("SELECT last_insert_rowid() AS id")
                .get_result::<LastInsertRowId>(conn)
                .map(|row| row.id)?;

            self.update_daily_stats_with_conn(conn, &today, &event.strategy, |daily, strategy| {
                daily.trades_opened += 1;
                daily.total_volume += decimal_to_f32(event.size);
                strategy.trades_opened += 1;
            })?;

            Ok::<i32, diesel::result::Error>(id)
        });

        match id {
            Ok(id) => {
                debug!(id = id, strategy = %event.strategy, "Recorded trade open");
                Some(id)
            }
            Err(e) => {
                warn!(error = %e, "Trade-open transaction failed");
                None
            }
        }
    }

    /// Record a trade closing event.
    pub fn record_trade_close(&self, event: &TradeCloseEvent) {
        let now = Utc::now().to_rfc3339();
        let today = Utc::now().date_naive().to_string();

        let mut conn = match self.pool.get() {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, "Failed to get database connection");
                return;
            }
        };

        let trade: Option<TradeRow> = trades::table
            .filter(trades::id.eq(event.trade_id))
            .first(&mut conn)
            .ok();

        let strategy = trade
            .as_ref()
            .map(|t| t.strategy.clone())
            .unwrap_or_default();
        let profit = decimal_to_f32(event.realized_profit);
        let is_win = event.realized_profit > Decimal::ZERO;

        let result = diesel::update(trades::table.filter(trades::id.eq(event.trade_id)))
            .set((
                trades::realized_profit.eq(Some(profit)),
                trades::status.eq("closed"),
                trades::closed_at.eq(Some(&now)),
                trades::close_reason.eq(Some(&event.reason)),
            ))
            .execute(&mut conn);

        if let Err(e) = result {
            warn!(error = %e, "Failed to record trade close");
            return;
        }

        self.update_daily_stats(&today, &strategy, |daily, strat| {
            daily.trades_closed += 1;
            strat.trades_closed += 1;
            if is_win {
                daily.profit_realized += profit;
                daily.win_count += 1;
                strat.profit_realized += profit;
                strat.win_count += 1;
            } else {
                daily.loss_realized += profit.abs();
                daily.loss_count += 1;
                strat.loss_count += 1;
            }
        });

        debug!(trade_id = event.trade_id, profit = %event.realized_profit, "Recorded trade close");
    }

    /// Record a latency measurement sample.
    pub fn record_latency(&self, latency_ms: u32) {
        let today = Utc::now().date_naive().to_string();
        self.update_daily_stats(&today, "", |daily, _| {
            daily.latency_sum_ms += latency_ms as i32;
            daily.latency_count += 1;
        });
    }

    /// Update peak exposure if the current value is higher than recorded.
    pub fn update_peak_exposure(&self, exposure: Decimal) {
        let today = Utc::now().date_naive().to_string();
        let exp = decimal_to_f32(exposure);
        self.update_daily_stats(&today, "", |daily, _| {
            if exp > daily.peak_exposure {
                daily.peak_exposure = exp;
            }
        });
    }

    /// Retrieve aggregated statistics for a date range.
    #[must_use]
    pub fn get_summary(&self, from: NaiveDate, to: NaiveDate) -> StatsSummary {
        let mut conn = match self.pool.get() {
            Ok(c) => c,
            Err(_) => return StatsSummary::default(),
        };

        let rows: Vec<DailyStatsRow> = daily_stats::table
            .filter(daily_stats::date.ge(from.to_string()))
            .filter(daily_stats::date.le(to.to_string()))
            .load(&mut conn)
            .unwrap_or_default();

        summary_from_rows(&rows)
    }

    /// Retrieve today's aggregated statistics.
    #[must_use]
    pub fn get_today(&self) -> StatsSummary {
        let today = Utc::now().date_naive();
        self.get_summary(today, today)
    }

    /// Prune old records while preserving aggregated daily statistics.
    pub fn prune_old_records(&self, retention_days: u32) {
        let cutoff = Utc::now().date_naive() - chrono::Duration::days(i64::from(retention_days));
        let cutoff_str = cutoff.to_string();

        let mut conn = match self.pool.get() {
            Ok(c) => c,
            Err(_) => return,
        };

        let opp_deleted =
            diesel::delete(opportunities::table.filter(opportunities::detected_at.lt(&cutoff_str)))
                .execute(&mut conn)
                .unwrap_or(0);

        let trades_deleted =
            diesel::delete(trades::table.filter(trades::opened_at.lt(&cutoff_str)))
                .execute(&mut conn)
                .unwrap_or(0);

        if opp_deleted > 0 || trades_deleted > 0 {
            debug!(
                opportunities = opp_deleted,
                trades = trades_deleted,
                "Pruned old records"
            );
        }
    }

    fn update_daily_stats<F>(&self, date: &str, strategy: &str, updater: F)
    where
        F: FnOnce(&mut DailyStatsRow, &mut StrategyDailyStatsRow),
    {
        if let Ok(mut conn) = self.pool.get() {
            let _ = self.update_daily_stats_with_conn(&mut conn, date, strategy, updater);
        }
    }

    fn update_daily_stats_with_conn<F>(
        &self,
        conn: &mut SqliteConnection,
        date: &str,
        strategy: &str,
        updater: F,
    ) -> Result<(), diesel::result::Error>
    where
        F: FnOnce(&mut DailyStatsRow, &mut StrategyDailyStatsRow),
    {
        let mut daily: DailyStatsRow = daily_stats::table
            .filter(daily_stats::date.eq(date))
            .first(conn)
            .optional()?
            .unwrap_or_else(|| DailyStatsRow {
                date: date.to_string(),
                ..Default::default()
            });

        let mut strat = if strategy.is_empty() {
            StrategyDailyStatsRow::default()
        } else {
            strategy_daily_stats::table
                .filter(strategy_daily_stats::date.eq(date))
                .filter(strategy_daily_stats::strategy.eq(strategy))
                .first(conn)
                .optional()?
                .unwrap_or_else(|| StrategyDailyStatsRow {
                    date: date.to_string(),
                    strategy: strategy.to_string(),
                    ..Default::default()
                })
        };

        updater(&mut daily, &mut strat);

        diesel::replace_into(daily_stats::table)
            .values(&daily)
            .execute(conn)?;

        if !strategy.is_empty() {
            diesel::replace_into(strategy_daily_stats::table)
                .values(&strat)
                .execute(conn)?;
        }

        Ok(())
    }
}

impl crate::port::outbound::stats::StatsRecorder for SqliteRecorder {
    fn record_opportunity(&self, event: &RecordedOpportunity) -> Option<i32> {
        SqliteRecorder::record_opportunity(self, event)
    }

    fn record_trade_open(&self, event: &TradeOpenEvent) -> Option<i32> {
        SqliteRecorder::record_trade_open(self, event)
    }

    fn record_trade_close(&self, event: &TradeCloseEvent) {
        SqliteRecorder::record_trade_close(self, event)
    }

    fn record_latency(&self, latency_ms: u32) {
        SqliteRecorder::record_latency(self, latency_ms)
    }

    fn update_peak_exposure(&self, exposure: Decimal) {
        SqliteRecorder::update_peak_exposure(self, exposure)
    }

    fn get_summary(&self, from: NaiveDate, to: NaiveDate) -> StatsSummary {
        SqliteRecorder::get_summary(self, from, to)
    }

    fn get_today(&self) -> StatsSummary {
        SqliteRecorder::get_today(self)
    }
}

/// Create a statistics recorder from a database connection pool.
#[must_use]
pub fn create_recorder(
    pool: Pool<ConnectionManager<SqliteConnection>>,
) -> Arc<dyn crate::port::outbound::stats::StatsRecorder> {
    Arc::new(SqliteRecorder::new(pool))
}

/// Export daily statistics to CSV format.
#[must_use]
pub fn export_daily_csv(
    pool: &Pool<ConnectionManager<SqliteConnection>>,
    from: NaiveDate,
    to: NaiveDate,
) -> String {
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return String::new(),
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

/// Retrieve recent opportunity records.
#[must_use]
pub fn recent_opportunities(
    pool: &Pool<ConnectionManager<SqliteConnection>>,
    limit: i64,
) -> Vec<OpportunitySummary> {
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
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

/// Build a statistics summary from daily stats rows.
#[must_use]
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
