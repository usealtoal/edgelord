//! Persistence ports for relations and clusters.
//!
//! Defines storage traits for persisting discovered market relations and
//! computed clusters.
//!
//! # Overview
//!
//! - [`RelationStore`]: CRUD operations for market relations
//! - [`ClusterStore`]: CRUD operations for market clusters

use std::future::Future;

use crate::domain::{cluster::Cluster, id::ClusterId, id::RelationId, relation::Relation};
use crate::error::Result;

/// Storage port for market relations.
///
/// Implementations persist inferred relations between markets for use in
/// cross-market arbitrage detection.
///
/// # Thread Safety
///
/// Implementations must be thread-safe (`Send + Sync`).
///
/// # Errors
///
/// All methods return [`Result`] for storage operation failures.
pub trait RelationStore: Send + Sync {
    /// Save a relation, replacing any existing relation with the same ID.
    ///
    /// # Arguments
    ///
    /// * `relation` - Relation to persist.
    ///
    /// # Errors
    ///
    /// Returns an error if the storage operation fails.
    fn save(&self, relation: &Relation) -> impl Future<Output = Result<()>> + Send;

    /// Retrieve a relation by its identifier.
    ///
    /// # Arguments
    ///
    /// * `id` - Relation identifier.
    ///
    /// Returns `None` if no relation exists with this ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the storage operation fails.
    fn get(&self, id: &RelationId) -> impl Future<Output = Result<Option<Relation>>> + Send;

    /// Delete a relation by its identifier.
    ///
    /// # Arguments
    ///
    /// * `id` - Relation identifier.
    ///
    /// Returns `true` if a relation was deleted, `false` if no relation
    /// existed with this ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the storage operation fails.
    fn delete(&self, id: &RelationId) -> impl Future<Output = Result<bool>> + Send;

    /// List all stored relations.
    ///
    /// # Arguments
    ///
    /// * `include_expired` - Whether to include expired relations.
    ///
    /// # Errors
    ///
    /// Returns an error if the storage operation fails.
    fn list(&self, include_expired: bool) -> impl Future<Output = Result<Vec<Relation>>> + Send;

    /// Delete all expired relations.
    ///
    /// Returns the number of relations deleted.
    ///
    /// # Errors
    ///
    /// Returns an error if the storage operation fails.
    fn prune_expired(&self) -> impl Future<Output = Result<usize>> + Send;
}

/// Storage port for market clusters.
///
/// Implementations persist computed clusters of related markets.
///
/// # Thread Safety
///
/// Implementations must be thread-safe (`Send + Sync`).
///
/// # Errors
///
/// All methods return [`Result`] for storage operation failures.
pub trait ClusterStore: Send + Sync {
    /// Save a cluster, replacing any existing cluster with the same ID.
    ///
    /// # Arguments
    ///
    /// * `cluster` - Cluster to persist.
    ///
    /// # Errors
    ///
    /// Returns an error if the storage operation fails.
    fn save(&self, cluster: &Cluster) -> impl Future<Output = Result<()>> + Send;

    /// Retrieve a cluster by its identifier.
    ///
    /// # Arguments
    ///
    /// * `id` - Cluster identifier.
    ///
    /// Returns `None` if no cluster exists with this ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the storage operation fails.
    fn get(&self, id: &ClusterId) -> impl Future<Output = Result<Option<Cluster>>> + Send;

    /// Delete a cluster by its identifier.
    ///
    /// # Arguments
    ///
    /// * `id` - Cluster identifier.
    ///
    /// Returns `true` if a cluster was deleted, `false` if no cluster
    /// existed with this ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the storage operation fails.
    fn delete(&self, id: &ClusterId) -> impl Future<Output = Result<bool>> + Send;

    /// List all stored clusters.
    ///
    /// # Errors
    ///
    /// Returns an error if the storage operation fails.
    fn list(&self) -> impl Future<Output = Result<Vec<Cluster>>> + Send;
}
