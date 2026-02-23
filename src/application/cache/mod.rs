//! Runtime caches and trackers used by application services.
//!
//! Provides thread-safe, in-memory storage for frequently accessed data:
//!
//! - [`book::BookCache`]: Order book snapshots with optional update notifications
//! - [`cluster::ClusterCache`]: Relation clusters with TTL-based expiration
//! - [`position::PositionTracker`]: Open and closed position tracking

pub mod book;
pub mod cluster;
pub mod position;
