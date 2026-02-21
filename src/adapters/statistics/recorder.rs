//! Statistics recording and aggregation service.
//!
//! Persists opportunities, trades, and daily aggregates to the database
//! for historical analysis and CLI reporting.

use chrono::{NaiveDate, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::OptionalExtension;
use diesel::SqliteConnection;
use rust_decimal::Decimal;

use super::convert::decimal_to_f32;
use super::query::{summary_for_range, summary_for_today};
use super::stat::{RecordedOpportunity, StatsSummary, TradeCloseEvent, TradeOpenEvent};
use tracing::{debug, warn};

use crate::adapters::stores::db;
use crate::adapters::stores::db::model::{
    DailyStatsRow, NewOpportunityRow, NewTradeRow, StrategyDailyStatsRow, TradeRow,
};
use crate::adapters::stores::db::schema::{
    daily_stats, opportunities, strategy_daily_stats, trades,
};

/// Statistics recorder for persisting events to the database.
pub struct StatsRecorder {
    pool: Pool<ConnectionManager<SqliteConnection>>,
}

/// Helper struct for querying last_insert_rowid().
#[derive(QueryableByName)]
struct LastInsertRowId {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    #[diesel(column_name = "id")]
    id: i32,
}

impl StatsRecorder {
    /// Create a new stats recorder.
    #[must_use]
    pub fn new(pool: Pool<ConnectionManager<SqliteConnection>>) -> Self {
        Self { pool }
    }

    /// Record an opportunity detection.
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

        let mut conn = match self.pool.get() {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, "Failed to get database connection for stats");
                return None;
            }
        };

        if let Err(e) = db::configure_sqlite_connection(&mut conn) {
            warn!(error = %e, "Failed to configure SQLite connection");
        }

        // Execute insert and daily stats update in a transaction
        let id = conn.transaction(|conn| {
            // Insert opportunity
            diesel::insert_into(opportunities::table)
                .values(&row)
                .execute(conn)
                .map_err(|e| {
                    warn!(error = %e, "Failed to record opportunity");
                    e
                })?;

            // Get the inserted ID using last_insert_rowid()
            // Note: SQLite doesn't support RETURNING, so we use last_insert_rowid()
            // Must be called immediately after INSERT and before any other operations
            let id: i32 = diesel::sql_query("SELECT last_insert_rowid() AS id")
                .get_result::<LastInsertRowId>(conn)
                .map(|row| row.id)
                .map_err(|e| {
                    warn!(error = %e, "Failed to fetch inserted opportunity ID");
                    e
                })?;

            // Update daily stats within the same transaction
            self.update_daily_stats_with_conn(conn, &today, &event.strategy, |daily, strategy| {
                daily.opportunities_detected += 1;
                strategy.opportunities_detected += 1;
                if event.executed {
                    daily.opportunities_executed += 1;
                    strategy.opportunities_executed += 1;
                } else if event.rejected_reason.is_some() {
                    daily.opportunities_rejected += 1;
                }
            })
            .map_err(|e| {
                warn!(error = %e, "Failed to update daily stats for opportunity");
                e
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

    /// Record a trade opening.
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

        let mut conn = match self.pool.get() {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, "Failed to get database connection for stats");
                return None;
            }
        };

        if let Err(e) = db::configure_sqlite_connection(&mut conn) {
            warn!(error = %e, "Failed to configure SQLite connection");
        }

        let id = conn.transaction(|conn| {
            diesel::insert_into(trades::table)
                .values(&row)
                .execute(conn)
                .map_err(|e| {
                    warn!(error = %e, "Failed to record trade open");
                    e
                })?;

            let id: i32 = diesel::sql_query("SELECT last_insert_rowid() AS id")
                .get_result::<LastInsertRowId>(conn)
                .map(|row| row.id)
                .map_err(|e| {
                    warn!(error = %e, "Failed to fetch inserted trade ID");
                    e
                })?;

            self.update_daily_stats_with_conn(conn, &today, &event.strategy, |daily, strategy| {
                daily.trades_opened += 1;
                daily.total_volume += decimal_to_f32(event.size);
                strategy.trades_opened += 1;
            })
            .map_err(|e| {
                warn!(error = %e, "Failed to update daily stats for trade open");
                e
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

    /// Record a trade closing.
    pub fn record_trade_close(&self, event: &TradeCloseEvent) {
        let now = Utc::now().to_rfc3339();
        let today = Utc::now().date_naive().to_string();

        let mut conn = match self.pool.get() {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, "Failed to get database connection for stats");
                return;
            }
        };

        // Get the trade to find strategy
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

        // Update trade record
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

        // Update daily stats
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

    /// Record latency sample.
    pub fn record_latency(&self, latency_ms: u32) {
        let today = Utc::now().date_naive().to_string();
        self.update_daily_stats(&today, "", |daily, _| {
            daily.latency_sum_ms += latency_ms as i32;
            daily.latency_count += 1;
        });
    }

    /// Update peak exposure if current is higher.
    pub fn update_peak_exposure(&self, exposure: Decimal) {
        let today = Utc::now().date_naive().to_string();
        let exp = decimal_to_f32(exposure);
        self.update_daily_stats(&today, "", |daily, _| {
            if exp > daily.peak_exposure {
                daily.peak_exposure = exp;
            }
        });
    }

    /// Get summary for a date range.
    pub fn get_summary(&self, from: NaiveDate, to: NaiveDate) -> StatsSummary {
        summary_for_range(&self.pool, from, to)
    }

    /// Get today's summary.
    pub fn get_today(&self) -> StatsSummary {
        summary_for_today(&self.pool)
    }

    /// Helper to update daily stats atomically.
    fn update_daily_stats<F>(&self, date: &str, strategy: &str, updater: F)
    where
        F: FnOnce(&mut DailyStatsRow, &mut StrategyDailyStatsRow),
    {
        let mut conn = match self.pool.get() {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, "Failed to get database connection for daily stats");
                return;
            }
        };
        let _ = self.update_daily_stats_with_conn(&mut conn, date, strategy, updater);
    }

    /// Helper to update daily stats with an existing connection (for use in transactions).
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
        // Upsert daily stats
        let mut daily: DailyStatsRow = daily_stats::table
            .filter(daily_stats::date.eq(date))
            .first(conn)
            .optional()?
            .unwrap_or_else(|| DailyStatsRow {
                date: date.to_string(),
                ..Default::default()
            });

        // Upsert strategy stats (skip if no strategy)
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

        // Save daily stats
        diesel::replace_into(daily_stats::table)
            .values(&daily)
            .execute(conn)?;

        // Save strategy stats (if applicable)
        if !strategy.is_empty() {
            diesel::replace_into(strategy_daily_stats::table)
                .values(&strat)
                .execute(conn)?;
        }

        Ok(())
    }

    /// Prune old raw records, keeping aggregated daily stats.
    ///
    /// Deletes opportunities and trades older than `retention_days`.
    /// Daily stats are never pruned.
    pub fn prune_old_records(&self, retention_days: u32) {
        let cutoff = Utc::now().date_naive() - chrono::Duration::days(i64::from(retention_days));
        let cutoff_str = cutoff.to_string();

        let mut conn = match self.pool.get() {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, "Failed to get database connection for pruning");
                return;
            }
        };

        // Delete old opportunities
        let opp_deleted =
            diesel::delete(opportunities::table.filter(opportunities::detected_at.lt(&cutoff_str)))
                .execute(&mut conn)
                .unwrap_or(0);

        // Delete old trades (cascade would be nice but SQLite support varies)
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
}
