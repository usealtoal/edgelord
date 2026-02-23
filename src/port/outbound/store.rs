//! Persistence ports for relations and clusters.

use std::future::Future;

use crate::domain::{cluster::Cluster, id::ClusterId, id::RelationId, relation::Relation};
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
