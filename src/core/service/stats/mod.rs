//! Statistics recording and aggregation service.
//!
//! Persists opportunities, trades, and daily aggregates to the database
//! for historical analysis and CLI reporting.

use std::sync::Arc;

use chrono::{NaiveDate, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use rust_decimal::Decimal;
use tracing::{debug, warn};

use crate::core::db::model::{
    DailyStatsRow, NewOpportunityRow, NewTradeRow, OpportunityRow, StrategyDailyStatsRow, TradeRow,
};
use crate::core::db::schema::{daily_stats, opportunities, strategy_daily_stats, trades};

/// Recorded opportunity for stats tracking.
#[derive(Debug, Clone)]
pub struct RecordedOpportunity {
    pub strategy: String,
    pub market_ids: Vec<String>,
    pub edge: Decimal,
    pub expected_profit: Decimal,
    pub executed: bool,
    pub rejected_reason: Option<String>,
}

/// Recorded trade open event.
#[derive(Debug, Clone)]
pub struct TradeOpenEvent {
    pub opportunity_id: i32,
    pub strategy: String,
    pub market_ids: Vec<String>,
    pub legs: Vec<TradeLeg>,
    pub size: Decimal,
    pub expected_profit: Decimal,
}

/// A single leg of a trade.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TradeLeg {
    pub token_id: String,
    pub side: String,
    pub price: Decimal,
    pub size: Decimal,
}

/// Recorded trade close event.
#[derive(Debug, Clone)]
pub struct TradeCloseEvent {
    pub trade_id: i32,
    pub realized_profit: Decimal,
    pub reason: String,
}

/// Summary statistics for a time period.
#[derive(Debug, Clone, Default)]
pub struct StatsSummary {
    pub opportunities_detected: i64,
    pub opportunities_executed: i64,
    pub opportunities_rejected: i64,
    pub trades_opened: i64,
    pub trades_closed: i64,
    pub profit_realized: Decimal,
    pub loss_realized: Decimal,
    pub win_count: i64,
    pub loss_count: i64,
    pub total_volume: Decimal,
}

impl StatsSummary {
    /// Calculate win rate as a percentage.
    #[must_use]
    pub fn win_rate(&self) -> Option<f64> {
        let total = self.win_count + self.loss_count;
        if total == 0 {
            None
        } else {
            Some(self.win_count as f64 / total as f64 * 100.0)
        }
    }

    /// Calculate net profit.
    #[must_use]
    pub fn net_profit(&self) -> Decimal {
        self.profit_realized - self.loss_realized
    }
}

/// Statistics recorder for persisting events to the database.
pub struct StatsRecorder {
    pool: Pool<ConnectionManager<SqliteConnection>>,
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
                warn!("Failed to get DB connection for stats: {e}");
                return None;
            }
        };

        // Insert opportunity
        let result = diesel::insert_into(opportunities::table)
            .values(&row)
            .execute(&mut conn);

        if let Err(e) = result {
            warn!("Failed to record opportunity: {e}");
            return None;
        }

        // Get the inserted ID
        let id: Option<i32> = opportunities::table
            .select(diesel::dsl::max(opportunities::id))
            .first(&mut conn)
            .ok()
            .flatten();

        // Update daily stats
        self.update_daily_stats(&today, &event.strategy, |daily, strategy| {
            daily.opportunities_detected += 1;
            strategy.opportunities_detected += 1;
            if event.executed {
                daily.opportunities_executed += 1;
                strategy.opportunities_executed += 1;
            } else if event.rejected_reason.is_some() {
                daily.opportunities_rejected += 1;
            }
        });

        debug!(id = ?id, strategy = %event.strategy, "Recorded opportunity");
        id
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
                warn!("Failed to get DB connection for stats: {e}");
                return None;
            }
        };

        let result = diesel::insert_into(trades::table)
            .values(&row)
            .execute(&mut conn);

        if let Err(e) = result {
            warn!("Failed to record trade open: {e}");
            return None;
        }

        let id: Option<i32> = trades::table
            .select(diesel::dsl::max(trades::id))
            .first(&mut conn)
            .ok()
            .flatten();

        // Update daily stats
        self.update_daily_stats(&today, &event.strategy, |daily, strategy| {
            daily.trades_opened += 1;
            daily.total_volume += decimal_to_f32(event.size);
            strategy.trades_opened += 1;
        });

        debug!(id = ?id, strategy = %event.strategy, "Recorded trade open");
        id
    }

    /// Record a trade closing.
    pub fn record_trade_close(&self, event: &TradeCloseEvent) {
        let now = Utc::now().to_rfc3339();
        let today = Utc::now().date_naive().to_string();

        let mut conn = match self.pool.get() {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to get DB connection for stats: {e}");
                return;
            }
        };

        // Get the trade to find strategy
        let trade: Option<TradeRow> = trades::table
            .filter(trades::id.eq(event.trade_id))
            .first(&mut conn)
            .ok();

        let strategy = trade.as_ref().map(|t| t.strategy.clone()).unwrap_or_default();
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
            warn!("Failed to record trade close: {e}");
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
        let mut conn = match self.pool.get() {
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

    /// Get today's summary.
    pub fn get_today(&self) -> StatsSummary {
        let today = Utc::now().date_naive();
        self.get_summary(today, today)
    }

    /// Helper to update daily stats atomically.
    fn update_daily_stats<F>(&self, date: &str, strategy: &str, updater: F)
    where
        F: FnOnce(&mut DailyStatsRow, &mut StrategyDailyStatsRow),
    {
        let mut conn = match self.pool.get() {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to get DB connection for daily stats: {e}");
                return;
            }
        };

        // Upsert daily stats
        let mut daily: DailyStatsRow = daily_stats::table
            .filter(daily_stats::date.eq(date))
            .first(&mut conn)
            .unwrap_or_else(|_| DailyStatsRow {
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
                .first(&mut conn)
                .unwrap_or_else(|_| StrategyDailyStatsRow {
                    date: date.to_string(),
                    strategy: strategy.to_string(),
                    ..Default::default()
                })
        };

        updater(&mut daily, &mut strat);

        // Save daily stats
        let _ = diesel::replace_into(daily_stats::table)
            .values(&daily)
            .execute(&mut conn);

        // Save strategy stats (if applicable)
        if !strategy.is_empty() {
            let _ = diesel::replace_into(strategy_daily_stats::table)
                .values(&strat)
                .execute(&mut conn);
        }
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
                warn!("Failed to get DB connection for pruning: {e}");
                return;
            }
        };

        // Delete old opportunities
        let opp_deleted = diesel::delete(
            opportunities::table.filter(opportunities::detected_at.lt(&cutoff_str)),
        )
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

    /// Export daily stats to CSV format.
    pub fn export_daily_csv(&self, from: NaiveDate, to: NaiveDate) -> String {
        let mut conn = match self.pool.get() {
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
    pub fn recent_opportunities(&self, limit: i64) -> Vec<OpportunitySummary> {
        let mut conn = match self.pool.get() {
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
}

/// Summary of an opportunity for display.
#[derive(Debug, Clone)]
pub struct OpportunitySummary {
    pub id: i32,
    pub strategy: String,
    pub edge: Decimal,
    pub expected_profit: Decimal,
    pub executed: bool,
    pub rejected_reason: Option<String>,
    pub detected_at: String,
}

fn decimal_to_f32(d: Decimal) -> f32 {
    use rust_decimal::prelude::ToPrimitive;
    d.to_f32().unwrap_or(0.0)
}

fn f32_to_decimal(f: f32) -> Decimal {
    use rust_decimal::prelude::FromPrimitive;
    Decimal::from_f32(f).unwrap_or(Decimal::ZERO)
}

/// Create a stats recorder from a database pool.
#[must_use]
pub fn create_recorder(pool: Pool<ConnectionManager<SqliteConnection>>) -> Arc<StatsRecorder> {
    Arc::new(StatsRecorder::new(pool))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn stats_summary_win_rate_with_trades() {
        let summary = StatsSummary {
            win_count: 7,
            loss_count: 3,
            ..Default::default()
        };
        assert!((summary.win_rate().unwrap() - 70.0).abs() < 0.001);
    }

    #[test]
    fn stats_summary_win_rate_no_trades() {
        let summary = StatsSummary::default();
        assert!(summary.win_rate().is_none());
    }

    #[test]
    fn stats_summary_net_profit() {
        let summary = StatsSummary {
            profit_realized: dec!(100),
            loss_realized: dec!(30),
            ..Default::default()
        };
        assert_eq!(summary.net_profit(), dec!(70));
    }

    #[test]
    fn decimal_conversion_roundtrip() {
        let d = dec!(123.45);
        let f = decimal_to_f32(d);
        let back = f32_to_decimal(f);
        // f32 precision loss is acceptable
        assert!((back - d).abs() < dec!(0.01));
    }
}
