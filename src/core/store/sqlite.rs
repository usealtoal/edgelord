//! SQLite store implementation using Diesel.

use chrono::{DateTime, Utc};
use diesel::prelude::*;

use super::RelationStore;
use crate::core::db::model::RelationRow;
use crate::core::db::schema::relations;
use crate::core::db::DbPool;
use crate::core::domain::{MarketId, Relation, RelationId, RelationKind};
use crate::error::{Error, Result};

/// SQLite-backed relation store.
pub struct SqliteRelationStore {
    pool: DbPool,
}

impl SqliteRelationStore {
    /// Create a new SQLite relation store.
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
    use crate::core::db::create_pool;
    use crate::core::domain::MarketId;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn setup_test_db() -> DbPool {
        let pool = create_pool(":memory:").expect("Failed to create pool");
        let mut conn = pool.get().expect("Failed to get connection");
        conn.run_pending_migrations(MIGRATIONS)
            .expect("Failed to run migrations");
        pool
    }

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
}
