//! Database model types for Diesel ORM.

use diesel::prelude::*;

use super::schema::{clusters, daily_stats, opportunities, relations, strategy_daily_stats, trades};

/// Database row for a relation.
#[derive(Queryable, Selectable, Insertable, Debug, Clone)]
#[diesel(table_name = relations)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct RelationRow {
    pub id: String,
    pub kind: String,
    pub confidence: f32,
    pub reasoning: String,
    pub inferred_at: String,
    pub expires_at: String,
    pub market_ids: String,
}

/// Database row for a cluster.
#[derive(Queryable, Selectable, Insertable, Debug, Clone)]
#[diesel(table_name = clusters)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ClusterRow {
    pub id: String,
    pub market_ids: String,
    pub relation_ids: String,
    pub constraints_json: String,
    pub updated_at: String,
}

/// Database row for an opportunity (insertable).
#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = opportunities)]
pub struct NewOpportunityRow {
    pub strategy: String,
    pub market_ids: String,
    pub edge: f32,
    pub expected_profit: f32,
    pub detected_at: String,
    pub executed: i32,
    pub rejected_reason: Option<String>,
}

/// Database row for an opportunity (queryable).
#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = opportunities)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct OpportunityRow {
    pub id: Option<i32>,
    pub strategy: String,
    pub market_ids: String,
    pub edge: f32,
    pub expected_profit: f32,
    pub detected_at: String,
    pub executed: i32,
    pub rejected_reason: Option<String>,
}

/// Database row for a trade (insertable).
#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = trades)]
pub struct NewTradeRow {
    pub opportunity_id: i32,
    pub strategy: String,
    pub market_ids: String,
    pub legs: String,
    pub size: f32,
    pub expected_profit: f32,
    pub status: String,
    pub opened_at: String,
}

/// Database row for a trade (queryable).
#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = trades)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TradeRow {
    pub id: Option<i32>,
    pub opportunity_id: i32,
    pub strategy: String,
    pub market_ids: String,
    pub legs: String,
    pub size: f32,
    pub expected_profit: f32,
    pub realized_profit: Option<f32>,
    pub status: String,
    pub opened_at: String,
    pub closed_at: Option<String>,
    pub close_reason: Option<String>,
}

/// Database row for daily stats.
#[derive(Queryable, Selectable, Insertable, AsChangeset, Debug, Clone, Default)]
#[diesel(table_name = daily_stats)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct DailyStatsRow {
    pub date: String,
    pub opportunities_detected: i32,
    pub opportunities_executed: i32,
    pub opportunities_rejected: i32,
    pub trades_opened: i32,
    pub trades_closed: i32,
    pub profit_realized: f32,
    pub loss_realized: f32,
    pub win_count: i32,
    pub loss_count: i32,
    pub total_volume: f32,
    pub peak_exposure: f32,
    pub latency_sum_ms: i32,
    pub latency_count: i32,
}

/// Database row for per-strategy daily stats.
#[derive(Queryable, Selectable, Insertable, AsChangeset, Debug, Clone, Default)]
#[diesel(table_name = strategy_daily_stats)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct StrategyDailyStatsRow {
    pub date: String,
    pub strategy: String,
    pub opportunities_detected: i32,
    pub opportunities_executed: i32,
    pub trades_opened: i32,
    pub trades_closed: i32,
    pub profit_realized: f32,
    pub win_count: i32,
    pub loss_count: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relation_row_is_insertable() {
        // Type check - if this compiles, the Insertable derive works
        let _row = RelationRow {
            id: "test".to_string(),
            kind: "{}".to_string(),
            confidence: 0.9,
            reasoning: "test".to_string(),
            inferred_at: "2026-01-01T00:00:00Z".to_string(),
            expires_at: "2026-01-02T00:00:00Z".to_string(),
            market_ids: "[]".to_string(),
        };
    }

    #[test]
    fn cluster_row_is_insertable() {
        let _row = ClusterRow {
            id: "test".to_string(),
            market_ids: "[]".to_string(),
            relation_ids: "[]".to_string(),
            constraints_json: "[]".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        };
    }

    #[test]
    fn new_opportunity_row_is_insertable() {
        let _row = NewOpportunityRow {
            strategy: "single_condition".to_string(),
            market_ids: "[\"abc\"]".to_string(),
            edge: 0.05,
            expected_profit: 1.50,
            detected_at: "2026-01-01T00:00:00Z".to_string(),
            executed: 0,
            rejected_reason: None,
        };
    }

    #[test]
    fn new_trade_row_is_insertable() {
        let _row = NewTradeRow {
            opportunity_id: 1,
            strategy: "single_condition".to_string(),
            market_ids: "[\"abc\"]".to_string(),
            legs: "[]".to_string(),
            size: 100.0,
            expected_profit: 5.0,
            status: "open".to_string(),
            opened_at: "2026-01-01T00:00:00Z".to_string(),
        };
    }

    #[test]
    fn daily_stats_row_default() {
        let row = DailyStatsRow {
            date: "2026-01-01".to_string(),
            ..Default::default()
        };
        assert_eq!(row.opportunities_detected, 0);
        assert_eq!(row.profit_realized, 0.0);
    }
}
