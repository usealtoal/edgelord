//! Cluster cache for relation inference results.

use std::collections::HashMap;

use chrono::{Duration, Utc};
use parking_lot::RwLock;

use crate::core::domain::{Cluster, ClusterId, MarketId, Relation};

/// Cache for relation clusters with TTL support.
#[derive(Debug)]
pub struct ClusterCache {
    /// Clusters by ID.
    clusters: RwLock<HashMap<ClusterId, Cluster>>,
    /// Market to cluster mapping for fast lookup.
    market_index: RwLock<HashMap<MarketId, ClusterId>>,
    /// Time-to-live for cached entries.
    ttl: Duration,
}

impl ClusterCache {
    /// Create a new cluster cache with the given TTL.
    pub fn new(ttl: Duration) -> Self {
        Self {
            clusters: RwLock::new(HashMap::new()),
            market_index: RwLock::new(HashMap::new()),
            ttl,
        }
    }

    /// Get cluster containing a market.
    #[must_use]
    pub fn get_for_market(&self, market_id: &MarketId) -> Option<Cluster> {
        let index = self.market_index.read();
        let cluster_id = index.get(market_id)?;
        self.get(cluster_id)
    }

    /// Get cluster by ID.
    #[must_use]
    pub fn get(&self, cluster_id: &ClusterId) -> Option<Cluster> {
        let clusters = self.clusters.read();
        let cluster = clusters.get(cluster_id)?;

        // Check expiration
        if cluster.updated_at + self.ttl < Utc::now() {
            return None;
        }

        Some(cluster.clone())
    }

    /// Check if a market has any relations.
    #[must_use]
    pub fn has_relations(&self, market_id: &MarketId) -> bool {
        self.get_for_market(market_id).is_some()
    }

    /// Insert or update a cluster.
    pub fn put(&self, cluster: Cluster) {
        let cluster_id = cluster.id.clone();

        // Update market index
        {
            let mut index = self.market_index.write();
            for market_id in &cluster.markets {
                index.insert(market_id.clone(), cluster_id.clone());
            }
        }

        // Insert cluster
        self.clusters.write().insert(cluster_id, cluster);
    }

    /// Insert relations and build clusters from them.
    pub fn put_relations(&self, relations: Vec<Relation>) {
        if relations.is_empty() {
            return;
        }

        // Build cluster from relations
        let cluster = Cluster::from_relations(relations);
        self.put(cluster);
    }

    /// Invalidate all clusters containing a market.
    pub fn invalidate(&self, market_id: &MarketId) {
        let cluster_id = {
            let index = self.market_index.read();
            index.get(market_id).cloned()
        };

        if let Some(id) = cluster_id {
            self.remove(&id);
        }
    }

    /// Remove a cluster by ID.
    pub fn remove(&self, cluster_id: &ClusterId) {
        let cluster = self.clusters.write().remove(cluster_id);

        if let Some(c) = cluster {
            let mut index = self.market_index.write();
            for market_id in &c.markets {
                index.remove(market_id);
            }
        }
    }

    /// Get all valid (non-expired) clusters.
    #[must_use]
    pub fn all_clusters(&self) -> Vec<Cluster> {
        let now = Utc::now();
        self.clusters
            .read()
            .values()
            .filter(|c| c.updated_at + self.ttl >= now)
            .cloned()
            .collect()
    }

    /// Prune expired clusters. Returns count removed.
    pub fn prune_expired(&self) -> usize {
        let now = Utc::now();
        let expired: Vec<ClusterId> = {
            self.clusters
                .read()
                .iter()
                .filter(|(_, c)| c.updated_at + self.ttl < now)
                .map(|(id, _)| id.clone())
                .collect()
        };

        for id in &expired {
            self.remove(id);
        }

        expired.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::domain::RelationKind;

    fn sample_relation() -> Relation {
        Relation::new(
            RelationKind::MutuallyExclusive {
                markets: vec![MarketId::new("m1"), MarketId::new("m2")],
            },
            0.9,
            "test".to_string(),
        )
    }

    #[test]
    fn cache_roundtrip() {
        let cache = ClusterCache::new(Duration::hours(1));
        let cluster = Cluster::from_relations(vec![sample_relation()]);
        let id = cluster.id.clone();

        cache.put(cluster);
        let loaded = cache.get(&id);
        assert!(loaded.is_some());
    }

    #[test]
    fn market_index_works() {
        let cache = ClusterCache::new(Duration::hours(1));
        cache.put_relations(vec![sample_relation()]);

        assert!(cache.has_relations(&MarketId::new("m1")));
        assert!(cache.has_relations(&MarketId::new("m2")));
        assert!(!cache.has_relations(&MarketId::new("m3")));
    }

    #[test]
    fn invalidate_removes_cluster() {
        let cache = ClusterCache::new(Duration::hours(1));
        cache.put_relations(vec![sample_relation()]);

        assert!(cache.has_relations(&MarketId::new("m1")));
        cache.invalidate(&MarketId::new("m1"));
        assert!(!cache.has_relations(&MarketId::new("m1")));
        assert!(!cache.has_relations(&MarketId::new("m2"))); // Same cluster
    }

    #[test]
    fn expired_clusters_not_returned() {
        let cache = ClusterCache::new(Duration::seconds(-1)); // Already expired
        cache.put_relations(vec![sample_relation()]);

        assert!(!cache.has_relations(&MarketId::new("m1")));
    }
}
