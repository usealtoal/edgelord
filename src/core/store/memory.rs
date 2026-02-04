//! In-memory store implementation for testing.

use std::collections::HashMap;

use chrono::Utc;
use parking_lot::RwLock;

use super::{ClusterStore, RelationStore};
use crate::core::domain::{Cluster, ClusterId, Relation, RelationId};
use crate::error::Result;

/// In-memory store for testing purposes.
#[derive(Debug, Default)]
pub struct MemoryStore {
    relations: RwLock<HashMap<RelationId, Relation>>,
    clusters: RwLock<HashMap<ClusterId, Cluster>>,
}

impl MemoryStore {
    /// Create a new empty memory store.
    pub fn new() -> Self {
        Self::default()
    }
}

impl RelationStore for MemoryStore {
    async fn save(&self, relation: &Relation) -> Result<()> {
        self.relations
            .write()
            .insert(relation.id.clone(), relation.clone());
        Ok(())
    }

    async fn get(&self, id: &RelationId) -> Result<Option<Relation>> {
        Ok(self.relations.read().get(id).cloned())
    }

    async fn delete(&self, id: &RelationId) -> Result<bool> {
        Ok(self.relations.write().remove(id).is_some())
    }

    async fn list(&self, include_expired: bool) -> Result<Vec<Relation>> {
        let now = Utc::now();
        let relations = self.relations.read();
        Ok(relations
            .values()
            .filter(|r| include_expired || r.expires_at > now)
            .cloned()
            .collect())
    }

    async fn prune_expired(&self) -> Result<usize> {
        let now = Utc::now();
        let mut relations = self.relations.write();
        let before = relations.len();
        relations.retain(|_, r| r.expires_at > now);
        Ok(before - relations.len())
    }
}

impl ClusterStore for MemoryStore {
    async fn save(&self, cluster: &Cluster) -> Result<()> {
        self.clusters
            .write()
            .insert(cluster.id.clone(), cluster.clone());
        Ok(())
    }

    async fn get(&self, id: &ClusterId) -> Result<Option<Cluster>> {
        Ok(self.clusters.read().get(id).cloned())
    }

    async fn delete(&self, id: &ClusterId) -> Result<bool> {
        Ok(self.clusters.write().remove(id).is_some())
    }

    async fn list(&self) -> Result<Vec<Cluster>> {
        Ok(self.clusters.read().values().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::domain::{MarketId, RelationKind};

    fn make_relation(suffix: &str) -> Relation {
        Relation::new(
            RelationKind::MutuallyExclusive {
                markets: vec![
                    MarketId::new(&format!("m1{suffix}")),
                    MarketId::new(&format!("m2{suffix}")),
                ],
            },
            0.9,
            "test".to_string(),
        )
    }

    #[tokio::test]
    async fn prune_expired_removes_old_relations() {
        let store = MemoryStore::new();

        let mut expired = make_relation("a");
        expired.expires_at = Utc::now() - chrono::Duration::hours(1);
        RelationStore::save(&store, &expired).await.unwrap();

        let valid = make_relation("b");
        RelationStore::save(&store, &valid).await.unwrap();

        let pruned = RelationStore::prune_expired(&store).await.unwrap();
        assert_eq!(pruned, 1);

        let remaining = RelationStore::list(&store, true).await.unwrap();
        assert_eq!(remaining.len(), 1);
    }

    #[tokio::test]
    async fn list_filters_expired_by_default() {
        let store = MemoryStore::new();

        let mut expired = make_relation("a");
        expired.expires_at = Utc::now() - chrono::Duration::hours(1);
        expired.reasoning = "expired".to_string();
        RelationStore::save(&store, &expired).await.unwrap();

        let mut valid = make_relation("b");
        valid.reasoning = "valid".to_string();
        RelationStore::save(&store, &valid).await.unwrap();

        let active = RelationStore::list(&store, false).await.unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].reasoning, "valid");

        let all = RelationStore::list(&store, true).await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn cluster_crud_operations() {
        let store = MemoryStore::new();

        let relation = make_relation("");
        let cluster = Cluster::from_relations(vec![relation]);
        let id = cluster.id.clone();

        ClusterStore::save(&store, &cluster).await.unwrap();
        let loaded = ClusterStore::get(&store, &id).await.unwrap().unwrap();
        assert_eq!(loaded.id, id);

        let all = ClusterStore::list(&store).await.unwrap();
        assert_eq!(all.len(), 1);

        assert!(ClusterStore::delete(&store, &id).await.unwrap());
        assert!(ClusterStore::get(&store, &id).await.unwrap().is_none());
    }
}
