//! Persistence layer with pluggable storage backends.
//!
//! This module re-exports from `crate::adapters::stores` for backward compatibility.

pub use crate::adapters::stores::{ClusterStore, MemoryStore, RelationStore, SqliteRelationStore};
