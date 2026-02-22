//! Persistence layer with pluggable storage backends.

pub mod db;
mod memory;
mod sqlite;

pub use memory::MemoryStore;
pub use sqlite::SqliteRelationStore;

use std::future::Future;

use crate::domain::{Cluster, ClusterId, Relation, RelationId};
use crate::error::Result;

/// Storage operations for relations.
pub trait RelationStore: Send + Sync {
    /// Save a relation, replacing if it exists.
    fn save(&self, relation: &Relation) -> impl Future<Output = Result<()>> + Send;

    /// Get a relation by ID.
    fn get(&self, id: &RelationId) -> impl Future<Output = Result<Option<Relation>>> + Send;

    /// Delete a relation by ID.
    fn delete(&self, id: &RelationId) -> impl Future<Output = Result<bool>> + Send;

    /// List all relations, optionally filtering expired ones.
    fn list(&self, include_expired: bool) -> impl Future<Output = Result<Vec<Relation>>> + Send;

    /// Delete all expired relations. Returns count deleted.
    fn prune_expired(&self) -> impl Future<Output = Result<usize>> + Send;
}

/// Storage operations for clusters.
pub trait ClusterStore: Send + Sync {
    /// Save a cluster, replacing if it exists.
    fn save(&self, cluster: &Cluster) -> impl Future<Output = Result<()>> + Send;

    /// Get a cluster by ID.
    fn get(&self, id: &ClusterId) -> impl Future<Output = Result<Option<Cluster>>> + Send;

    /// Delete a cluster by ID.
    fn delete(&self, id: &ClusterId) -> impl Future<Output = Result<bool>> + Send;

    /// List all clusters.
    fn list(&self) -> impl Future<Output = Result<Vec<Cluster>>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{MarketId, RelationKind};

    fn sample_relation() -> Relation {
        Relation::new(
            RelationKind::MutuallyExclusive {
                markets: vec![MarketId::new("m1"), MarketId::new("m2")],
            },
            0.9,
            "test reasoning".to_string(),
        )
    }

    #[tokio::test]
    async fn memory_store_relation_roundtrip() {
        let store = MemoryStore::new();
        let relation = sample_relation();
        let id = relation.id.clone();

        RelationStore::save(&store, &relation).await.unwrap();
        let loaded = RelationStore::get(&store, &id).await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().id, id);
    }

    #[tokio::test]
    async fn memory_store_relation_delete() {
        let store = MemoryStore::new();
        let relation = sample_relation();
        let id = relation.id.clone();

        RelationStore::save(&store, &relation).await.unwrap();
        assert!(RelationStore::delete(&store, &id).await.unwrap());
        assert!(RelationStore::get(&store, &id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn memory_store_relation_list() {
        let store = MemoryStore::new();
        let r1 = sample_relation();
        let r2 = sample_relation();

        RelationStore::save(&store, &r1).await.unwrap();
        RelationStore::save(&store, &r2).await.unwrap();

        let all = RelationStore::list(&store, true).await.unwrap();
        assert_eq!(all.len(), 2);
    }
}
