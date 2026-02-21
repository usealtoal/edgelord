//! Stateful caches and repositories for domain objects.
//!
//! DEPRECATED: This module is being phased out. Use `crate::runtime::cache` instead.

// Re-export from new location for backward compatibility
pub use crate::runtime::cache::{ClusterCache, OrderBookCache, OrderBookUpdate, PositionTracker};
