//! Database model types for Diesel ORM.

use diesel::prelude::*;

use super::schema::{
    clusters, daily_stats, opportunities, relations, strategy_daily_stats, trades,
};

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
    use crate::adapter::outbound::sqlite::database::connection::{create_pool, run_migrations};

    // -------------------------------------------------------------------------
    // Type construction tests
    // -------------------------------------------------------------------------

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

    // -------------------------------------------------------------------------
    // Default value tests
    // -------------------------------------------------------------------------

    #[test]
    fn daily_stats_row_all_defaults() {
        let row = DailyStatsRow::default();

        assert_eq!(row.date, "");
        assert_eq!(row.opportunities_detected, 0);
        assert_eq!(row.opportunities_executed, 0);
        assert_eq!(row.opportunities_rejected, 0);
        assert_eq!(row.trades_opened, 0);
        assert_eq!(row.trades_closed, 0);
        assert!((row.profit_realized - 0.0).abs() < 0.001);
        assert!((row.loss_realized - 0.0).abs() < 0.001);
        assert_eq!(row.win_count, 0);
        assert_eq!(row.loss_count, 0);
        assert!((row.total_volume - 0.0).abs() < 0.001);
        assert!((row.peak_exposure - 0.0).abs() < 0.001);
        assert_eq!(row.latency_sum_ms, 0);
        assert_eq!(row.latency_count, 0);
    }

    #[test]
    fn strategy_daily_stats_row_default() {
        let row = StrategyDailyStatsRow::default();

        assert_eq!(row.date, "");
        assert_eq!(row.strategy, "");
        assert_eq!(row.opportunities_detected, 0);
        assert_eq!(row.opportunities_executed, 0);
        assert_eq!(row.trades_opened, 0);
        assert_eq!(row.trades_closed, 0);
        assert!((row.profit_realized - 0.0).abs() < 0.001);
        assert_eq!(row.win_count, 0);
        assert_eq!(row.loss_count, 0);
    }

    // -------------------------------------------------------------------------
    // Clone tests
    // -------------------------------------------------------------------------

    #[test]
    fn relation_row_is_cloneable() {
        let row = RelationRow {
            id: "test".to_string(),
            kind: "{}".to_string(),
            confidence: 0.9,
            reasoning: "test".to_string(),
            inferred_at: "2026-01-01T00:00:00Z".to_string(),
            expires_at: "2026-01-02T00:00:00Z".to_string(),
            market_ids: "[]".to_string(),
        };

        let cloned = row.clone();

        assert_eq!(cloned.id, row.id);
        assert_eq!(cloned.confidence, row.confidence);
    }

    #[test]
    fn daily_stats_row_is_cloneable() {
        let row = DailyStatsRow {
            date: "2026-01-15".to_string(),
            opportunities_detected: 100,
            profit_realized: 500.0,
            ..Default::default()
        };

        let cloned = row.clone();

        assert_eq!(cloned.date, "2026-01-15");
        assert_eq!(cloned.opportunities_detected, 100);
    }

    // -------------------------------------------------------------------------
    // Debug trait tests
    // -------------------------------------------------------------------------

    #[test]
    fn relation_row_is_debuggable() {
        let row = RelationRow {
            id: "test-id".to_string(),
            kind: "{}".to_string(),
            confidence: 0.9,
            reasoning: "test".to_string(),
            inferred_at: "2026-01-01T00:00:00Z".to_string(),
            expires_at: "2026-01-02T00:00:00Z".to_string(),
            market_ids: "[]".to_string(),
        };

        let debug_str = format!("{:?}", row);

        assert!(debug_str.contains("test-id"));
        assert!(debug_str.contains("0.9"));
    }

    #[test]
    fn cluster_row_is_debuggable() {
        let row = ClusterRow {
            id: "cluster-1".to_string(),
            market_ids: "[\"m1\",\"m2\"]".to_string(),
            relation_ids: "[]".to_string(),
            constraints_json: "[]".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        };

        let debug_str = format!("{:?}", row);

        assert!(debug_str.contains("cluster-1"));
    }

    // -------------------------------------------------------------------------
    // Database roundtrip tests
    // -------------------------------------------------------------------------

    #[test]
    fn relation_row_roundtrip_with_db() {
        let pool = create_pool(":memory:").unwrap();
        run_migrations(&pool).unwrap();
        let mut conn = pool.get().unwrap();

        let row = RelationRow {
            id: "rel-1".to_string(),
            kind: r#"{"type":"mutually_exclusive","markets":["m1","m2"]}"#.to_string(),
            confidence: 0.95,
            reasoning: "Test reasoning".to_string(),
            inferred_at: "2026-01-01T00:00:00Z".to_string(),
            expires_at: "2026-01-02T00:00:00Z".to_string(),
            market_ids: r#"["m1","m2"]"#.to_string(),
        };

        diesel::insert_into(relations::table)
            .values(&row)
            .execute(&mut conn)
            .unwrap();

        let loaded: RelationRow = relations::table.find("rel-1").first(&mut conn).unwrap();

        assert_eq!(loaded.id, "rel-1");
        assert!((loaded.confidence - 0.95).abs() < 0.001);
        assert_eq!(loaded.reasoning, "Test reasoning");
    }

    #[test]
    fn cluster_row_roundtrip_with_db() {
        let pool = create_pool(":memory:").unwrap();
        run_migrations(&pool).unwrap();
        let mut conn = pool.get().unwrap();

        let row = ClusterRow {
            id: "cluster-1".to_string(),
            market_ids: r#"["m1","m2","m3"]"#.to_string(),
            relation_ids: r#"["rel-1","rel-2"]"#.to_string(),
            constraints_json: r#"[{"coefficients":[1,1,1],"sense":"leq","rhs":1}]"#.to_string(),
            updated_at: "2026-01-01T12:00:00Z".to_string(),
        };

        diesel::insert_into(clusters::table)
            .values(&row)
            .execute(&mut conn)
            .unwrap();

        let loaded: ClusterRow = clusters::table.find("cluster-1").first(&mut conn).unwrap();

        assert_eq!(loaded.id, "cluster-1");
        assert!(loaded.market_ids.contains("m1"));
        assert!(loaded.market_ids.contains("m2"));
        assert!(loaded.market_ids.contains("m3"));
    }

    #[test]
    fn opportunity_row_roundtrip_with_db() {
        let pool = create_pool(":memory:").unwrap();
        run_migrations(&pool).unwrap();
        let mut conn = pool.get().unwrap();

        let new_row = NewOpportunityRow {
            strategy: "single_condition".to_string(),
            market_ids: r#"["market-abc"]"#.to_string(),
            edge: 0.05,
            expected_profit: 10.0,
            detected_at: "2026-01-15T10:30:00Z".to_string(),
            executed: 1,
            rejected_reason: None,
        };

        diesel::insert_into(opportunities::table)
            .values(&new_row)
            .execute(&mut conn)
            .unwrap();

        let loaded: OpportunityRow = opportunities::table
            .order(opportunities::id.desc())
            .first(&mut conn)
            .unwrap();

        assert!(loaded.id.is_some());
        assert_eq!(loaded.strategy, "single_condition");
        assert!((loaded.edge - 0.05).abs() < 0.001);
        assert_eq!(loaded.executed, 1);
    }

    #[test]
    fn trade_row_roundtrip_with_db() {
        let pool = create_pool(":memory:").unwrap();
        run_migrations(&pool).unwrap();
        let mut conn = pool.get().unwrap();

        // First create an opportunity (foreign key constraint)
        let opp = NewOpportunityRow {
            strategy: "test".to_string(),
            market_ids: "[]".to_string(),
            edge: 0.05,
            expected_profit: 5.0,
            detected_at: "2026-01-01T00:00:00Z".to_string(),
            executed: 1,
            rejected_reason: None,
        };
        diesel::insert_into(opportunities::table)
            .values(&opp)
            .execute(&mut conn)
            .unwrap();

        let new_trade = NewTradeRow {
            opportunity_id: 1,
            strategy: "single_condition".to_string(),
            market_ids: r#"["market-1"]"#.to_string(),
            legs: r#"[{"token_id":"t1","side":"buy","price":0.5,"size":100}]"#.to_string(),
            size: 100.0,
            expected_profit: 5.0,
            status: "open".to_string(),
            opened_at: "2026-01-15T10:30:00Z".to_string(),
        };

        diesel::insert_into(trades::table)
            .values(&new_trade)
            .execute(&mut conn)
            .unwrap();

        let loaded: TradeRow = trades::table
            .order(trades::id.desc())
            .first(&mut conn)
            .unwrap();

        assert!(loaded.id.is_some());
        assert_eq!(loaded.opportunity_id, 1);
        assert_eq!(loaded.status, "open");
        assert!((loaded.size - 100.0).abs() < 0.001);
        assert!(loaded.realized_profit.is_none());
        assert!(loaded.closed_at.is_none());
    }

    #[test]
    fn daily_stats_row_roundtrip_with_db() {
        let pool = create_pool(":memory:").unwrap();
        run_migrations(&pool).unwrap();
        let mut conn = pool.get().unwrap();

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

        diesel::insert_into(daily_stats::table)
            .values(&row)
            .execute(&mut conn)
            .unwrap();

        let loaded: DailyStatsRow = daily_stats::table
            .find("2026-01-15")
            .first(&mut conn)
            .unwrap();

        assert_eq!(loaded.date, "2026-01-15");
        assert_eq!(loaded.opportunities_detected, 100);
        assert_eq!(loaded.win_count, 15);
        assert!((loaded.profit_realized - 500.0).abs() < 0.001);
    }

    #[test]
    fn strategy_daily_stats_row_roundtrip_with_db() {
        let pool = create_pool(":memory:").unwrap();
        run_migrations(&pool).unwrap();
        let mut conn = pool.get().unwrap();

        let row = StrategyDailyStatsRow {
            date: "2026-01-15".to_string(),
            strategy: "single_condition".to_string(),
            opportunities_detected: 50,
            opportunities_executed: 25,
            trades_opened: 15,
            trades_closed: 12,
            profit_realized: 250.0,
            win_count: 10,
            loss_count: 2,
        };

        diesel::insert_into(strategy_daily_stats::table)
            .values(&row)
            .execute(&mut conn)
            .unwrap();

        let loaded: StrategyDailyStatsRow = strategy_daily_stats::table
            .filter(strategy_daily_stats::date.eq("2026-01-15"))
            .filter(strategy_daily_stats::strategy.eq("single_condition"))
            .first(&mut conn)
            .unwrap();

        assert_eq!(loaded.date, "2026-01-15");
        assert_eq!(loaded.strategy, "single_condition");
        assert_eq!(loaded.opportunities_detected, 50);
    }

    // -------------------------------------------------------------------------
    // Edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn opportunity_row_with_rejected_reason() {
        let pool = create_pool(":memory:").unwrap();
        run_migrations(&pool).unwrap();
        let mut conn = pool.get().unwrap();

        let row = NewOpportunityRow {
            strategy: "test".to_string(),
            market_ids: "[]".to_string(),
            edge: 0.05,
            expected_profit: 5.0,
            detected_at: "2026-01-01T00:00:00Z".to_string(),
            executed: 0,
            rejected_reason: Some("risk_limit_exceeded".to_string()),
        };

        diesel::insert_into(opportunities::table)
            .values(&row)
            .execute(&mut conn)
            .unwrap();

        let loaded: OpportunityRow = opportunities::table
            .order(opportunities::id.desc())
            .first(&mut conn)
            .unwrap();

        assert_eq!(loaded.executed, 0);
        assert_eq!(
            loaded.rejected_reason,
            Some("risk_limit_exceeded".to_string())
        );
    }

    #[test]
    fn trade_row_with_close_data() {
        let pool = create_pool(":memory:").unwrap();
        run_migrations(&pool).unwrap();
        let mut conn = pool.get().unwrap();

        // Create opportunity first
        let opp = NewOpportunityRow {
            strategy: "test".to_string(),
            market_ids: "[]".to_string(),
            edge: 0.05,
            expected_profit: 5.0,
            detected_at: "2026-01-01T00:00:00Z".to_string(),
            executed: 1,
            rejected_reason: None,
        };
        diesel::insert_into(opportunities::table)
            .values(&opp)
            .execute(&mut conn)
            .unwrap();

        // Create trade
        let trade = NewTradeRow {
            opportunity_id: 1,
            strategy: "test".to_string(),
            market_ids: "[]".to_string(),
            legs: "[]".to_string(),
            size: 100.0,
            expected_profit: 5.0,
            status: "open".to_string(),
            opened_at: "2026-01-01T10:00:00Z".to_string(),
        };
        diesel::insert_into(trades::table)
            .values(&trade)
            .execute(&mut conn)
            .unwrap();

        // Update with close data
        diesel::update(trades::table.filter(trades::id.eq(1)))
            .set((
                trades::status.eq("closed"),
                trades::realized_profit.eq(Some(10.5f32)),
                trades::closed_at.eq(Some("2026-01-01T12:00:00Z")),
                trades::close_reason.eq(Some("market_settled")),
            ))
            .execute(&mut conn)
            .unwrap();

        let loaded: TradeRow = trades::table.first(&mut conn).unwrap();

        assert_eq!(loaded.status, "closed");
        assert!((loaded.realized_profit.unwrap() - 10.5).abs() < 0.001);
        assert_eq!(loaded.closed_at, Some("2026-01-01T12:00:00Z".to_string()));
        assert_eq!(loaded.close_reason, Some("market_settled".to_string()));
    }

    #[test]
    fn relation_row_with_special_characters() {
        let pool = create_pool(":memory:").unwrap();
        run_migrations(&pool).unwrap();
        let mut conn = pool.get().unwrap();

        let row = RelationRow {
            id: "rel-special".to_string(),
            kind: "{}".to_string(),
            confidence: 0.9,
            reasoning: "Special chars: 'quotes', \"double\", emoji 🎉, unicode: éàü".to_string(),
            inferred_at: "2026-01-01T00:00:00Z".to_string(),
            expires_at: "2026-01-02T00:00:00Z".to_string(),
            market_ids: "[]".to_string(),
        };

        diesel::insert_into(relations::table)
            .values(&row)
            .execute(&mut conn)
            .unwrap();

        let loaded: RelationRow = relations::table
            .find("rel-special")
            .first(&mut conn)
            .unwrap();

        assert!(loaded.reasoning.contains("🎉"));
        assert!(loaded.reasoning.contains("éàü"));
    }
}
