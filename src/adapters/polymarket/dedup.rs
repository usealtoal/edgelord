//! Message deduplication for Polymarket WebSocket messages.
//!
//! Implements the [`MessageDeduplicator`] trait to filter duplicate messages
//! from redundant WebSocket connections.

use std::time::{Duration, Instant};

use dashmap::DashMap;

use crate::runtime::exchange::{MarketEvent, MessageDeduplicator};
use crate::runtime::PolymarketDedupConfig;

/// Thread-safe message deduplicator for Polymarket.
///
/// Uses a concurrent hash map to track seen messages and detect duplicates
/// across multiple WebSocket connections.
pub struct PolymarketDeduplicator {
    /// Cache of seen message keys with their insertion time.
    cache: DashMap<String, Instant>,
    /// Time-to-live for cache entries.
    ttl: Duration,
    /// Maximum number of entries in the cache.
    max_entries: usize,
    /// Whether deduplication is enabled.
    enabled: bool,
}

impl PolymarketDeduplicator {
    /// Create a new deduplicator from configuration.
    #[must_use]
    pub fn new(config: &PolymarketDedupConfig) -> Self {
        Self {
            cache: DashMap::new(),
            ttl: Duration::from_secs(config.cache_ttl_secs),
            max_entries: config.max_cache_entries,
            enabled: config.enabled,
        }
    }

    /// Create a dedup key from a market event.
    ///
    /// The key combines the token ID with a hash of the order book content
    /// to uniquely identify each message.
    fn make_key(event: &MarketEvent) -> Option<String> {
        match event {
            MarketEvent::OrderBookSnapshot { token_id, book } => {
                let mut key = format!("snap:{}", token_id.as_str());

                // Add best bid/ask to the key for content-based dedup
                if let Some(bid) = book.best_bid() {
                    key.push_str(&format!(":b{}@{}", bid.size(), bid.price()));
                }
                if let Some(ask) = book.best_ask() {
                    key.push_str(&format!(":a{}@{}", ask.size(), ask.price()));
                }

                // Add order book depth for additional uniqueness
                key.push_str(&format!(":d{}:{}", book.bids().len(), book.asks().len()));

                Some(key)
            }
            MarketEvent::OrderBookDelta { token_id, book } => {
                let mut key = format!("delta:{}", token_id.as_str());

                // Add best bid/ask to the key for content-based dedup
                if let Some(bid) = book.best_bid() {
                    key.push_str(&format!(":b{}@{}", bid.size(), bid.price()));
                }
                if let Some(ask) = book.best_ask() {
                    key.push_str(&format!(":a{}@{}", ask.size(), ask.price()));
                }

                // Add order book depth for additional uniqueness
                key.push_str(&format!(":d{}:{}", book.bids().len(), book.asks().len()));

                Some(key)
            }
            // Connection and settlement events are not deduplicated
            MarketEvent::Connected
            | MarketEvent::Disconnected { .. }
            | MarketEvent::MarketSettled { .. } => None,
        }
    }
}

impl MessageDeduplicator for PolymarketDeduplicator {
    fn is_duplicate(&self, event: &MarketEvent) -> bool {
        // If disabled, nothing is a duplicate
        if !self.enabled {
            return false;
        }

        // Connection events are never duplicates
        let Some(key) = Self::make_key(event) else {
            return false;
        };

        let now = Instant::now();

        // Check if we've seen this key before
        if let Some(entry) = self.cache.get(&key) {
            let age = now.duration_since(*entry);
            if age < self.ttl {
                // Seen recently, this is a duplicate
                return true;
            }
            // Entry expired, fall through to insert new one
        }

        // Insert or update the entry
        self.cache.insert(key, now);

        // Trigger GC if we're over the limit
        if self.cache.len() > self.max_entries {
            self.gc();
        }

        false
    }

    fn gc(&self) {
        let now = Instant::now();
        let ttl = self.ttl;

        // Remove expired entries
        self.cache
            .retain(|_, inserted| now.duration_since(*inserted) < ttl);

        // If still over limit after TTL cleanup, remove oldest entries
        if self.cache.len() > self.max_entries {
            // Collect entries with their ages
            let mut entries: Vec<(String, Instant)> = self
                .cache
                .iter()
                .map(|entry| (entry.key().clone(), *entry.value()))
                .collect();

            // Sort by age (oldest first)
            entries.sort_by(|a, b| a.1.cmp(&b.1));

            // Remove oldest entries until we're under the limit
            let to_remove = entries.len().saturating_sub(self.max_entries);
            for (key, _) in entries.into_iter().take(to_remove) {
                self.cache.remove(&key);
            }
        }
    }

    fn cache_size(&self) -> usize {
        self.cache.len()
    }

    fn exchange_name(&self) -> &'static str {
        "polymarket"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{OrderBook, PriceLevel, TokenId};
    use rust_decimal_macros::dec;

    fn test_config() -> PolymarketDedupConfig {
        PolymarketDedupConfig {
            enabled: true,
            cache_ttl_secs: 5,
            max_cache_entries: 100,
            ..Default::default()
        }
    }

    fn make_snapshot(token_id: &str, bid_price: f64, ask_price: f64) -> MarketEvent {
        let token = TokenId::new(token_id);
        let bids = vec![PriceLevel::new(
            rust_decimal::Decimal::from_f64_retain(bid_price).unwrap(),
            dec!(100),
        )];
        let asks = vec![PriceLevel::new(
            rust_decimal::Decimal::from_f64_retain(ask_price).unwrap(),
            dec!(100),
        )];
        let book = OrderBook::with_levels(token.clone(), bids, asks);
        MarketEvent::OrderBookSnapshot {
            token_id: token,
            book,
        }
    }

    fn make_delta(token_id: &str, bid_price: f64, ask_price: f64) -> MarketEvent {
        let token = TokenId::new(token_id);
        let bids = vec![PriceLevel::new(
            rust_decimal::Decimal::from_f64_retain(bid_price).unwrap(),
            dec!(100),
        )];
        let asks = vec![PriceLevel::new(
            rust_decimal::Decimal::from_f64_retain(ask_price).unwrap(),
            dec!(100),
        )];
        let book = OrderBook::with_levels(token.clone(), bids, asks);
        MarketEvent::OrderBookDelta {
            token_id: token,
            book,
        }
    }

    #[test]
    fn test_new_deduplicator() {
        let config = test_config();
        let dedup = PolymarketDeduplicator::new(&config);

        assert_eq!(dedup.cache_size(), 0);
        assert_eq!(dedup.exchange_name(), "polymarket");
    }

    #[test]
    fn test_first_event_not_duplicate() {
        let config = test_config();
        let dedup = PolymarketDeduplicator::new(&config);
        let event = make_snapshot("token-1", 0.45, 0.55);

        assert!(!dedup.is_duplicate(&event));
        assert_eq!(dedup.cache_size(), 1);
    }

    #[test]
    fn test_same_event_is_duplicate() {
        let config = test_config();
        let dedup = PolymarketDeduplicator::new(&config);
        let event = make_snapshot("token-1", 0.45, 0.55);

        assert!(!dedup.is_duplicate(&event));
        assert!(dedup.is_duplicate(&event));
        assert_eq!(dedup.cache_size(), 1);
    }

    #[test]
    fn test_different_tokens_not_duplicate() {
        let config = test_config();
        let dedup = PolymarketDeduplicator::new(&config);
        let event1 = make_snapshot("token-1", 0.45, 0.55);
        let event2 = make_snapshot("token-2", 0.45, 0.55);

        assert!(!dedup.is_duplicate(&event1));
        assert!(!dedup.is_duplicate(&event2));
        assert_eq!(dedup.cache_size(), 2);
    }

    #[test]
    fn test_different_prices_not_duplicate() {
        let config = test_config();
        let dedup = PolymarketDeduplicator::new(&config);
        let event1 = make_snapshot("token-1", 0.45, 0.55);
        let event2 = make_snapshot("token-1", 0.46, 0.55);

        assert!(!dedup.is_duplicate(&event1));
        assert!(!dedup.is_duplicate(&event2));
        assert_eq!(dedup.cache_size(), 2);
    }

    #[test]
    fn test_snapshot_and_delta_different() {
        let config = test_config();
        let dedup = PolymarketDeduplicator::new(&config);
        let snapshot = make_snapshot("token-1", 0.45, 0.55);
        let delta = make_delta("token-1", 0.45, 0.55);

        assert!(!dedup.is_duplicate(&snapshot));
        assert!(!dedup.is_duplicate(&delta));
        assert_eq!(dedup.cache_size(), 2);
    }

    #[test]
    fn test_connection_events_not_deduplicated() {
        let config = test_config();
        let dedup = PolymarketDeduplicator::new(&config);

        assert!(!dedup.is_duplicate(&MarketEvent::Connected));
        assert!(!dedup.is_duplicate(&MarketEvent::Connected));
        assert!(!dedup.is_duplicate(&MarketEvent::Disconnected {
            reason: "test".into()
        }));

        // Connection events don't add to cache
        assert_eq!(dedup.cache_size(), 0);
    }

    #[test]
    fn test_disabled_dedup_never_duplicate() {
        let config = PolymarketDedupConfig {
            enabled: false,
            ..Default::default()
        };
        let dedup = PolymarketDeduplicator::new(&config);
        let event = make_snapshot("token-1", 0.45, 0.55);

        assert!(!dedup.is_duplicate(&event));
        assert!(!dedup.is_duplicate(&event));
        assert!(!dedup.is_duplicate(&event));

        // Nothing added when disabled
        assert_eq!(dedup.cache_size(), 0);
    }

    #[test]
    fn test_gc_removes_expired_entries() {
        let config = PolymarketDedupConfig {
            enabled: true,
            cache_ttl_secs: 0, // Immediate expiration
            max_cache_entries: 100,
            ..Default::default()
        };
        let dedup = PolymarketDeduplicator::new(&config);

        // Add some events
        let event1 = make_snapshot("token-1", 0.45, 0.55);
        let event2 = make_snapshot("token-2", 0.45, 0.55);

        assert!(!dedup.is_duplicate(&event1));
        assert!(!dedup.is_duplicate(&event2));
        assert_eq!(dedup.cache_size(), 2);

        // GC should remove expired entries
        dedup.gc();
        assert_eq!(dedup.cache_size(), 0);
    }

    #[test]
    fn test_gc_respects_max_entries() {
        let config = PolymarketDedupConfig {
            enabled: true,
            cache_ttl_secs: 60, // Long TTL
            max_cache_entries: 2,
            ..Default::default()
        };
        let dedup = PolymarketDeduplicator::new(&config);

        // Add events up to limit
        for i in 0..5 {
            let event = make_snapshot(&format!("token-{}", i), 0.45, 0.55);
            dedup.is_duplicate(&event);
        }

        // GC should trim to max_entries
        dedup.gc();
        assert!(dedup.cache_size() <= 2);
    }

    #[test]
    fn test_make_key_includes_token_and_prices() {
        let event = make_snapshot("my-token", 0.45, 0.55);
        let key = PolymarketDeduplicator::make_key(&event).unwrap();

        assert!(key.contains("my-token"));
        assert!(key.contains("snap:"));
    }

    #[test]
    fn test_make_key_delta_prefix() {
        let event = make_delta("my-token", 0.45, 0.55);
        let key = PolymarketDeduplicator::make_key(&event).unwrap();

        assert!(key.contains("delta:"));
    }

    #[test]
    fn test_make_key_connection_events_none() {
        assert!(PolymarketDeduplicator::make_key(&MarketEvent::Connected).is_none());
        assert!(
            PolymarketDeduplicator::make_key(&MarketEvent::Disconnected {
                reason: "test".into()
            })
            .is_none()
        );
    }
}
