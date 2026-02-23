//! Message deduplication port for redundant WebSocket connections.
//!
//! Defines traits and configuration for filtering duplicate market events
//! when using multiple redundant connections to the same exchange.

use crate::port::outbound::exchange::MarketEvent;

/// Strategy for detecting duplicate messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DedupStrategy {
    /// Hash-based deduplication using a hash of message content.
    ///
    /// Fast and memory-efficient; suitable for most use cases.
    #[default]
    Hash,

    /// Timestamp-based deduplication using message timestamps.
    ///
    /// Relies on messages having consistent timestamps across connections.
    Timestamp,

    /// Content-based deduplication comparing full message content.
    ///
    /// Most accurate but highest memory usage.
    Content,
}

/// Configuration for message deduplication.
#[derive(Debug, Clone)]
pub struct DedupConfig {
    /// Whether deduplication is enabled.
    pub enabled: bool,

    /// Strategy for detecting duplicates.
    pub strategy: DedupStrategy,

    /// Time-to-live for cache entries in seconds.
    ///
    /// Entries older than this are eligible for garbage collection.
    pub cache_ttl_secs: u64,

    /// Maximum number of entries to retain in the cache.
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

/// Port for filtering duplicate market events.
///
/// Implementations track recently seen events and filter duplicates that
/// arrive from redundant WebSocket connections.
///
/// # Thread Safety
///
/// Implementations must be thread-safe (`Send + Sync`) as events may arrive
/// concurrently from multiple connections.
pub trait MessageDeduplicator: Send + Sync {
    /// Check if an event is a duplicate and record it for future checks.
    ///
    /// # Arguments
    ///
    /// * `event` - Market event to check.
    ///
    /// Returns `true` if this event has been seen recently and should be
    /// discarded.
    fn is_duplicate(&self, event: &MarketEvent) -> bool;

    /// Perform garbage collection on expired cache entries.
    ///
    /// Should be called periodically to prevent unbounded memory growth.
    fn gc(&self);

    /// Return the current number of entries in the deduplication cache.
    fn cache_size(&self) -> usize;

    /// Return the exchange name for logging and metrics.
    fn exchange_name(&self) -> &'static str;
}
