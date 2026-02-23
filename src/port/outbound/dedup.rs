//! Port for message deduplication strategies.

use crate::port::outbound::exchange::MarketEvent;

/// Strategy for detecting duplicate messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DedupStrategy {
    /// Hash-based deduplication using message content hash.
    #[default]
    Hash,
    /// Timestamp-based deduplication using message timestamps.
    Timestamp,
    /// Content-based deduplication comparing full message content.
    Content,
}

/// Configuration for message deduplication.
#[derive(Debug, Clone)]
pub struct DedupConfig {
    /// Whether deduplication is enabled.
    pub enabled: bool,
    /// Strategy to use for detecting duplicates.
    pub strategy: DedupStrategy,
    /// Time-to-live for cache entries in seconds.
    pub cache_ttl_secs: u64,
    /// Maximum number of entries to keep in the cache.
    pub max_cache_entries: usize,
}

impl Default for DedupConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            strategy: DedupStrategy::Hash,
            cache_ttl_secs: 5,
            max_cache_entries: 100_000,
        }
    }
}

/// Port for filtering duplicate market events from redundant connections.
pub trait MessageDeduplicator: Send + Sync {
    /// Check if an event is a duplicate and record it.
    fn is_duplicate(&self, event: &MarketEvent) -> bool;

    /// Garbage collect old entries from the cache.
    fn gc(&self);

    /// Get current cache size.
    fn cache_size(&self) -> usize;

    /// Exchange name associated with this deduplicator.
    fn exchange_name(&self) -> &'static str;
}
