//! Message deduplication for filtering duplicate messages from redundant connections.

use crate::port::MarketEvent;

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

/// Trait for filtering duplicate market events from redundant connections.
///
/// Implementations track seen messages and detect duplicates to prevent
/// processing the same market data multiple times when using redundant
/// exchange connections.
pub trait MessageDeduplicator: Send + Sync {
    /// Check if an event is a duplicate and record it.
    ///
    /// Returns `true` if the event has been seen before (is a duplicate),
    /// `false` if this is the first time seeing this event.
    ///
    /// This method should atomically check and record the event.
    fn is_duplicate(&self, event: &MarketEvent) -> bool;

    /// Garbage collect old entries from the cache.
    ///
    /// Removes entries that have exceeded the configured TTL or
    /// when the cache exceeds maximum size.
    fn gc(&self);

    /// Get the current number of entries in the cache.
    fn cache_size(&self) -> usize;

    /// Get the exchange name this deduplicator is associated with.
    fn exchange_name(&self) -> &'static str;
}
