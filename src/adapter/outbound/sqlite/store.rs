//! SQLite relation store implementation.
//!
//! Provides persistent storage for market relations using SQLite and Diesel ORM.

use chrono::{DateTime, Utc};
use diesel::prelude::*;

use crate::adapter::outbound::sqlite::database::connection::DbPool;
use crate::adapter::outbound::sqlite::database::model::RelationRow;
use crate::adapter::outbound::sqlite::database::schema::relations;
use crate::domain::{id::MarketId, id::RelationId, relation::Relation, relation::RelationKind};
use crate::error::{Error, Result};
use crate::port::outbound::store::RelationStore;

/// SQLite-backed relation store.
///
/// Implements the [`RelationStore`] trait for persistent storage of
/// inferred market relations.
pub struct SqliteRelationStore {
    /// Database connection pool.
    pool: DbPool,
}

impl SqliteRelationStore {
    /// Create a new SQLite relation store with the given connection pool.
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    fn to_row(relation: &Relation) -> Result<RelationRow> {
        Self::to_row_with(
            relation,
            |kind| serde_json::to_string(kind).map_err(|e| Error::Parse(e.to_string())),
            |market_ids| serde_json::to_string(market_ids).map_err(|e| Error::Parse(e.to_string())),
        )
    }

    fn to_row_with<F1, F2>(
        relation: &Relation,
        serialize_kind: F1,
        serialize_market_ids: F2,
    ) -> Result<RelationRow>
    where
        F1: FnOnce(&RelationKind) -> Result<String>,
        F2: FnOnce(&[&MarketId]) -> Result<String>,
    {
        let market_ids = relation.market_ids();
        Ok(RelationRow {
            id: relation.id.to_string(),
            kind: serialize_kind(&relation.kind)?,
            confidence: relation.confidence as f32,
            reasoning: relation.reasoning.clone(),
            inferred_at: relation.inferred_at.to_rfc3339(),
            expires_at: relation.expires_at.to_rfc3339(),
            market_ids: serialize_market_ids(&market_ids)?,
        })
    }

    fn from_row(row: RelationRow) -> Result<Relation> {
        let kind: RelationKind =
            serde_json::from_str(&row.kind).map_err(|e| Error::Parse(e.to_string()))?;
        let inferred_at: DateTime<Utc> = DateTime::parse_from_rfc3339(&row.inferred_at)
            .map_err(|e| Error::Parse(e.to_string()))?
            .with_timezone(&Utc);
        let expires_at: DateTime<Utc> = DateTime::parse_from_rfc3339(&row.expires_at)
            .map_err(|e| Error::Parse(e.to_string()))?
            .with_timezone(&Utc);

        Ok(Relation {
            id: RelationId::from(row.id),
            kind,
            confidence: f64::from(row.confidence),
            reasoning: row.reasoning,
            inferred_at,
            expires_at,
        })
    }
}

impl RelationStore for SqliteRelationStore {
    async fn save(&self, relation: &Relation) -> Result<()> {
        let row = Self::to_row(relation)?;
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::Connection(e.to_string()))?;

        diesel::replace_into(relations::table)
            .values(&row)
            .execute(&mut conn)
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn get(&self, id: &RelationId) -> Result<Option<Relation>> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::Connection(e.to_string()))?;

        let row: Option<RelationRow> = relations::table
            .find(id.to_string())
            .first(&mut conn)
            .optional()
            .map_err(|e| Error::Database(e.to_string()))?;

        row.map(Self::from_row).transpose()
    }

    async fn delete(&self, id: &RelationId) -> Result<bool> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::Connection(e.to_string()))?;

        let deleted = diesel::delete(relations::table.find(id.to_string()))
            .execute(&mut conn)
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(deleted > 0)
    }

    async fn list(&self, include_expired: bool) -> Result<Vec<Relation>> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::Connection(e.to_string()))?;

        let now = Utc::now().to_rfc3339();
        let rows: Vec<RelationRow> = if include_expired {
            relations::table
                .load(&mut conn)
                .map_err(|e| Error::Database(e.to_string()))?
        } else {
            relations::table
                .filter(relations::expires_at.gt(&now))
                .load(&mut conn)
                .map_err(|e| Error::Database(e.to_string()))?
        };

        rows.into_iter().map(Self::from_row).collect()
    }

    async fn prune_expired(&self) -> Result<usize> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::Connection(e.to_string()))?;

        let now = Utc::now().to_rfc3339();
        let deleted = diesel::delete(relations::table.filter(relations::expires_at.le(&now)))
            .execute(&mut conn)
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(deleted)
    }
}

impl SqliteRelationStore {
    #[cfg(test)]
    async fn save_with_serializers<F1, F2>(
        &self,
        relation: &Relation,
        serialize_kind: F1,
        serialize_market_ids: F2,
    ) -> Result<()>
    where
        F1: FnOnce(&RelationKind) -> Result<String>,
        F2: FnOnce(&[&MarketId]) -> Result<String>,
    {
        let row = Self::to_row_with(relation, serialize_kind, serialize_market_ids)?;
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::Connection(e.to_string()))?;

        diesel::replace_into(relations::table)
            .values(&row)
            .execute(&mut conn)
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::outbound::sqlite::database::connection::create_pool;
    use crate::domain::constraint::ConstraintSense;
    use crate::domain::id::MarketId;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn setup_test_db() -> DbPool {
        let pool = create_pool(":memory:").expect("Failed to create pool");
        let mut conn = pool.get().expect("Failed to get connection");
        conn.run_pending_migrations(MIGRATIONS)
            .expect("Failed to run migrations");
        pool
    }

    fn market(s: &str) -> MarketId {
        MarketId::new(s)
    }

    // -------------------------------------------------------------------------
    // Basic CRUD operations
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn sqlite_relation_roundtrip() {
        let pool = setup_test_db();
        let store = SqliteRelationStore::new(pool);

        let relation = Relation::new(
            RelationKind::MutuallyExclusive {
                markets: vec![MarketId::new("m1"), MarketId::new("m2")],
            },
            0.85,
            "Test reasoning".to_string(),
        );
        let id = relation.id.clone();

        store.save(&relation).await.unwrap();
        let loaded = store.get(&id).await.unwrap().unwrap();

        assert_eq!(loaded.id, id);
        assert!(
            (loaded.confidence - 0.85).abs() < 0.001,
            "confidence mismatch"
        );
        assert_eq!(loaded.reasoning, "Test reasoning");
    }

    #[tokio::test]
    async fn sqlite_relation_delete() {
        let pool = setup_test_db();
        let store = SqliteRelationStore::new(pool);

        let relation = Relation::new(
            RelationKind::MutuallyExclusive {
                markets: vec![MarketId::new("m1"), MarketId::new("m2")],
            },
            0.9,
            "test".to_string(),
        );
        let id = relation.id.clone();

        store.save(&relation).await.unwrap();
        assert!(store.delete(&id).await.unwrap());
        assert!(store.get(&id).await.unwrap().is_none());
        assert!(!store.delete(&id).await.unwrap()); // Already deleted
    }

    #[tokio::test]
    async fn sqlite_list_and_prune() {
        let pool = setup_test_db();
        let store = SqliteRelationStore::new(pool);

        // Create valid relation
        let valid = Relation::new(
            RelationKind::MutuallyExclusive {
                markets: vec![MarketId::new("m1"), MarketId::new("m2")],
            },
            0.9,
            "valid".to_string(),
        );
        store.save(&valid).await.unwrap();

        // Create expired relation
        let mut expired = Relation::new(
            RelationKind::MutuallyExclusive {
                markets: vec![MarketId::new("m3"), MarketId::new("m4")],
            },
            0.9,
            "expired".to_string(),
        );
        expired.expires_at = Utc::now() - chrono::Duration::hours(1);
        store.save(&expired).await.unwrap();

        // List active only
        let active = store.list(false).await.unwrap();
        assert_eq!(active.len(), 1);

        // List all
        let all = store.list(true).await.unwrap();
        assert_eq!(all.len(), 2);

        // Prune
        let pruned = store.prune_expired().await.unwrap();
        assert_eq!(pruned, 1);

        let remaining = store.list(true).await.unwrap();
        assert_eq!(remaining.len(), 1);
    }

    #[tokio::test]
    async fn save_returns_error_on_invalid_json() {
        let pool = setup_test_db();
        let store = SqliteRelationStore::new(pool);

        let relation = Relation::new(
            RelationKind::MutuallyExclusive {
                markets: vec![MarketId::new("m1"), MarketId::new("m2")],
            },
            0.85,
            "Test reasoning".to_string(),
        );
        let result = store
            .save_with_serializers(
                &relation,
                |_| Err(Error::Parse("forced kind error".to_string())),
                |_| Ok("[]".to_string()),
            )
            .await;

        assert!(
            matches!(result, Err(Error::Parse(_))),
            "Expected parse error when serialization fails"
        );

        let result = store
            .save_with_serializers(
                &relation,
                |_| Ok("{\"ok\":true}".to_string()),
                |_| Err(Error::Parse("forced market_ids error".to_string())),
            )
            .await;

        assert!(
            matches!(result, Err(Error::Parse(_))),
            "Expected parse error when market_ids serialization fails"
        );
    }

    // -------------------------------------------------------------------------
    // All RelationKind variants
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn roundtrip_implies_relation() {
        let pool = setup_test_db();
        let store = SqliteRelationStore::new(pool);

        let relation = Relation::new(
            RelationKind::Implies {
                if_yes: market("state-pa"),
                then_yes: market("national"),
            },
            0.95,
            "PA win implies national win".to_string(),
        );
        let id = relation.id.clone();

        store.save(&relation).await.unwrap();
        let loaded = store.get(&id).await.unwrap().unwrap();

        match &loaded.kind {
            RelationKind::Implies { if_yes, then_yes } => {
                assert_eq!(if_yes.as_str(), "state-pa");
                assert_eq!(then_yes.as_str(), "national");
            }
            _ => panic!("Expected Implies variant"),
        }
    }

    #[tokio::test]
    async fn roundtrip_exactly_one_relation() {
        let pool = setup_test_db();
        let store = SqliteRelationStore::new(pool);

        let relation = Relation::new(
            RelationKind::ExactlyOne {
                markets: vec![
                    market("candidate-a"),
                    market("candidate-b"),
                    market("other"),
                ],
            },
            0.99,
            "Exactly one candidate wins".to_string(),
        );
        let id = relation.id.clone();

        store.save(&relation).await.unwrap();
        let loaded = store.get(&id).await.unwrap().unwrap();

        match &loaded.kind {
            RelationKind::ExactlyOne { markets } => {
                assert_eq!(markets.len(), 3);
            }
            _ => panic!("Expected ExactlyOne variant"),
        }
    }

    #[tokio::test]
    async fn roundtrip_linear_relation() {
        let pool = setup_test_db();
        let store = SqliteRelationStore::new(pool);

        let relation = Relation::new(
            RelationKind::Linear {
                terms: vec![(market("m1"), dec!(0.5)), (market("m2"), dec!(0.5))],
                sense: ConstraintSense::LessEqual,
                rhs: dec!(1.0),
            },
            0.80,
            "Custom linear constraint".to_string(),
        );
        let id = relation.id.clone();

        store.save(&relation).await.unwrap();
        let loaded = store.get(&id).await.unwrap().unwrap();

        match &loaded.kind {
            RelationKind::Linear { terms, sense, rhs } => {
                assert_eq!(terms.len(), 2);
                assert_eq!(*sense, ConstraintSense::LessEqual);
                assert_eq!(*rhs, dec!(1.0));
            }
            _ => panic!("Expected Linear variant"),
        }
    }

    // -------------------------------------------------------------------------
    // Replace/upsert behavior
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn save_replaces_existing_relation() {
        let pool = setup_test_db();
        let store = SqliteRelationStore::new(pool);

        let mut relation = Relation::new(
            RelationKind::MutuallyExclusive {
                markets: vec![market("m1"), market("m2")],
            },
            0.80,
            "Original reasoning".to_string(),
        );
        let id = relation.id.clone();

        store.save(&relation).await.unwrap();

        // Update confidence and reasoning
        relation.confidence = 0.95;
        relation.reasoning = "Updated reasoning".to_string();
        store.save(&relation).await.unwrap();

        let loaded = store.get(&id).await.unwrap().unwrap();
        assert!((loaded.confidence - 0.95).abs() < 0.001);
        assert_eq!(loaded.reasoning, "Updated reasoning");

        // Verify only one relation exists
        let all = store.list(true).await.unwrap();
        assert_eq!(all.len(), 1);
    }

    // -------------------------------------------------------------------------
    // Edge cases
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn get_nonexistent_returns_none() {
        let pool = setup_test_db();
        let store = SqliteRelationStore::new(pool);

        let id = RelationId::new();
        let result = store.get(&id).await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn list_empty_database_returns_empty() {
        let pool = setup_test_db();
        let store = SqliteRelationStore::new(pool);

        let active = store.list(false).await.unwrap();
        let all = store.list(true).await.unwrap();

        assert!(active.is_empty());
        assert!(all.is_empty());
    }

    #[tokio::test]
    async fn prune_empty_database_returns_zero() {
        let pool = setup_test_db();
        let store = SqliteRelationStore::new(pool);

        let pruned = store.prune_expired().await.unwrap();

        assert_eq!(pruned, 0);
    }

    #[tokio::test]
    async fn save_relation_with_empty_market_list() {
        let pool = setup_test_db();
        let store = SqliteRelationStore::new(pool);

        let relation = Relation::new(
            RelationKind::MutuallyExclusive { markets: vec![] },
            0.5,
            "Empty markets".to_string(),
        );
        let id = relation.id.clone();

        store.save(&relation).await.unwrap();
        let loaded = store.get(&id).await.unwrap().unwrap();

        match &loaded.kind {
            RelationKind::MutuallyExclusive { markets } => {
                assert!(markets.is_empty());
            }
            _ => panic!("Expected MutuallyExclusive variant"),
        }
    }

    #[tokio::test]
    async fn save_relation_with_special_characters_in_reasoning() {
        let pool = setup_test_db();
        let store = SqliteRelationStore::new(pool);

        let relation = Relation::new(
            RelationKind::MutuallyExclusive {
                markets: vec![market("m1")],
            },
            0.9,
            "Reasoning with 'quotes', \"double quotes\", and émojis! 🎉".to_string(),
        );
        let id = relation.id.clone();

        store.save(&relation).await.unwrap();
        let loaded = store.get(&id).await.unwrap().unwrap();

        assert!(loaded.reasoning.contains("émojis"));
        assert!(loaded.reasoning.contains("🎉"));
    }

    // -------------------------------------------------------------------------
    // Concurrent access
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn concurrent_saves_do_not_corrupt_data() {
        let pool = setup_test_db();
        let store = Arc::new(SqliteRelationStore::new(pool));

        let mut handles = vec![];

        for i in 0..10 {
            let store_clone = Arc::clone(&store);
            let handle = tokio::spawn(async move {
                let relation = Relation::new(
                    RelationKind::MutuallyExclusive {
                        markets: vec![market(&format!("m{}", i))],
                    },
                    0.9,
                    format!("Relation {}", i),
                );
                store_clone.save(&relation).await.unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        let all = store.list(true).await.unwrap();
        assert_eq!(all.len(), 10);
    }

    #[tokio::test]
    async fn concurrent_reads_and_writes() {
        let pool = setup_test_db();
        let store = Arc::new(SqliteRelationStore::new(pool));

        // Pre-populate with some relations
        for i in 0..5 {
            let relation = Relation::new(
                RelationKind::MutuallyExclusive {
                    markets: vec![market(&format!("initial{}", i))],
                },
                0.9,
                format!("Initial {}", i),
            );
            store.save(&relation).await.unwrap();
        }

        let mut handles = vec![];

        // Mix of reads and writes
        for i in 0..20 {
            let store_clone = Arc::clone(&store);
            let handle = tokio::spawn(async move {
                if i % 2 == 0 {
                    // Write
                    let relation = Relation::new(
                        RelationKind::MutuallyExclusive {
                            markets: vec![market(&format!("new{}", i))],
                        },
                        0.9,
                        format!("New {}", i),
                    );
                    store_clone.save(&relation).await.unwrap();
                } else {
                    // Read
                    let _ = store_clone.list(true).await.unwrap();
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        // Should have initial 5 + 10 new ones (evens from 0..20)
        let all = store.list(true).await.unwrap();
        assert_eq!(all.len(), 15);
    }

    // -------------------------------------------------------------------------
    // Datetime serialization/deserialization
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn relation_timestamps_roundtrip_correctly() {
        let pool = setup_test_db();
        let store = SqliteRelationStore::new(pool);

        let relation = Relation::new(
            RelationKind::MutuallyExclusive {
                markets: vec![market("m1")],
            },
            0.9,
            "test".to_string(),
        )
        .with_ttl(chrono::Duration::hours(24));

        let id = relation.id.clone();
        let original_inferred = relation.inferred_at;
        let original_expires = relation.expires_at;

        store.save(&relation).await.unwrap();
        let loaded = store.get(&id).await.unwrap().unwrap();

        // Timestamps should be within 1 second of original
        assert!((loaded.inferred_at - original_inferred).num_seconds().abs() < 1);
        assert!((loaded.expires_at - original_expires).num_seconds().abs() < 1);
    }

    #[tokio::test]
    async fn prune_respects_exact_expiration_boundary() {
        let pool = setup_test_db();
        let store = SqliteRelationStore::new(pool);

        // Create relation that expires exactly now
        let mut just_expired = Relation::new(
            RelationKind::MutuallyExclusive {
                markets: vec![market("m1")],
            },
            0.9,
            "just expired".to_string(),
        );
        just_expired.expires_at = Utc::now();
        store.save(&just_expired).await.unwrap();

        // Create relation that expires in the future
        let not_expired = Relation::new(
            RelationKind::MutuallyExclusive {
                markets: vec![market("m2")],
            },
            0.9,
            "not expired".to_string(),
        );
        store.save(&not_expired).await.unwrap();

        // Wait a tiny bit to ensure the first one is definitely expired
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let pruned = store.prune_expired().await.unwrap();
        assert_eq!(pruned, 1);
    }
}
