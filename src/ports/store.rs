//! Store port for persistence operations.
//!
//! This module defines the traits for persisting domain objects
//! such as relations and clusters.

use std::future::Future;

use crate::domain::{Cluster, ClusterId, Relation, RelationId};
use crate::error::Result;

/// Storage operations for relations.
///
/// Relations represent logical dependencies between markets
/// (e.g., "A implies B", "exactly one of A, B, C").
///
/// # Implementation Notes
///
/// - Implementations must be thread-safe (`Send + Sync`)
/// - Methods return futures that can be awaited
/// - The `prune_expired` method should be called periodically to clean up stale data
pub trait Store: Send + Sync {
    /// Save a relation, replacing if it exists.
    fn save_relation(&self, relation: &Relation) -> impl Future<Output = Result<()>> + Send;

    /// Get a relation by ID.
    fn get_relation(
        &self,
        id: &RelationId,
    ) -> impl Future<Output = Result<Option<Relation>>> + Send;

    /// Delete a relation by ID. Returns true if the relation existed.
    fn delete_relation(&self, id: &RelationId) -> impl Future<Output = Result<bool>> + Send;

    /// List all relations, optionally including expired ones.
    fn list_relations(
        &self,
        include_expired: bool,
    ) -> impl Future<Output = Result<Vec<Relation>>> + Send;

    /// Delete all expired relations. Returns count deleted.
    fn prune_expired_relations(&self) -> impl Future<Output = Result<usize>> + Send;

    /// Save a cluster, replacing if it exists.
    fn save_cluster(&self, cluster: &Cluster) -> impl Future<Output = Result<()>> + Send;

    /// Get a cluster by ID.
    fn get_cluster(&self, id: &ClusterId) -> impl Future<Output = Result<Option<Cluster>>> + Send;

    /// Delete a cluster by ID. Returns true if the cluster existed.
    fn delete_cluster(&self, id: &ClusterId) -> impl Future<Output = Result<bool>> + Send;

    /// List all clusters.
    fn list_clusters(&self) -> impl Future<Output = Result<Vec<Cluster>>> + Send;
}
