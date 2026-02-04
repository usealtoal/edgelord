//! Priority-based subscription manager implementation.
//!
//! This module provides [`PrioritySubscriptionManager`], which manages market subscriptions
//! using a priority queue based on market scores. Higher-scoring markets are given
//! priority when expanding subscriptions.

use std::collections::{BinaryHeap, HashMap, HashSet};
use std::sync::RwLock;

use async_trait::async_trait;
use tracing::{debug, info, warn};

use super::subscription::{ConnectionEvent, SubscriptionManager};
use crate::core::domain::{MarketId, MarketScore, TokenId};
use crate::error::Result;

/// A priority-based subscription manager that maintains subscriptions
/// to the highest-scoring markets within resource constraints.
///
/// # Thread Safety
///
/// All internal state is protected by [`RwLock`] to allow safe concurrent access
/// from multiple async tasks.
///
/// # Priority Queue Behavior
///
/// Markets are ordered by their composite score (highest priority first).
/// When expanding, the highest-scoring markets are selected. When contracting,
/// the most recently added tokens are removed (LIFO) for simplicity.
///
/// # Example
///
/// ```
/// use edgelord::core::service::{PrioritySubscriptionManager, SubscriptionManager};
/// use edgelord::core::domain::{MarketId, MarketScore, ScoreFactors, TokenId};
///
/// let manager = PrioritySubscriptionManager::new(100);
///
/// // Register token mappings for markets
/// let market_id = MarketId::new("market-1");
/// let tokens = vec![TokenId::new("token-1a"), TokenId::new("token-1b")];
/// manager.register_market_tokens(market_id.clone(), tokens);
///
/// // Enqueue markets with scores
/// let factors = ScoreFactors::new(0.8, 0.7, 0.6, 0.5, 0.9);
/// let score = MarketScore::new(market_id, factors, 0.7);
/// manager.enqueue(vec![score]);
/// ```
pub struct PrioritySubscriptionManager {
    /// Priority queue of markets waiting to be subscribed.
    /// Higher composite scores have higher priority.
    pending: RwLock<BinaryHeap<MarketScore>>,

    /// Set of market IDs currently subscribed.
    active_markets: RwLock<HashSet<MarketId>>,

    /// List of active token subscriptions (ordered for LIFO removal).
    active_tokens: RwLock<Vec<TokenId>>,

    /// Mapping from market ID to its associated tokens.
    market_tokens: RwLock<HashMap<MarketId, Vec<TokenId>>>,

    /// Maximum number of subscriptions allowed.
    max_subscriptions: usize,
}

impl PrioritySubscriptionManager {
    /// Create a new priority subscription manager.
    ///
    /// # Arguments
    ///
    /// * `max_subscriptions` - Maximum number of token subscriptions to maintain
    ///
    /// # Example
    ///
    /// ```
    /// use edgelord::core::service::{PrioritySubscriptionManager, SubscriptionManager};
    ///
    /// let manager = PrioritySubscriptionManager::new(1000);
    /// assert_eq!(manager.max_subscriptions(), 1000);
    /// ```
    #[must_use]
    pub fn new(max_subscriptions: usize) -> Self {
        Self {
            pending: RwLock::new(BinaryHeap::new()),
            active_markets: RwLock::new(HashSet::new()),
            active_tokens: RwLock::new(Vec::new()),
            market_tokens: RwLock::new(HashMap::new()),
            max_subscriptions,
        }
    }

    /// Register the token mapping for a market.
    ///
    /// This associates a market ID with its constituent tokens. Must be called
    /// before enqueueing a market for subscription.
    ///
    /// # Arguments
    ///
    /// * `market_id` - The market identifier
    /// * `tokens` - The tokens associated with this market
    ///
    /// # Example
    ///
    /// ```
    /// use edgelord::core::service::PrioritySubscriptionManager;
    /// use edgelord::core::domain::{MarketId, TokenId};
    ///
    /// let manager = PrioritySubscriptionManager::new(100);
    /// let market_id = MarketId::new("market-1");
    /// let tokens = vec![TokenId::new("token-1a"), TokenId::new("token-1b")];
    ///
    /// manager.register_market_tokens(market_id.clone(), tokens);
    /// ```
    pub fn register_market_tokens(&self, market_id: MarketId, tokens: Vec<TokenId>) {
        let mut market_tokens = self.market_tokens.write().expect("lock poisoned");
        market_tokens.insert(market_id, tokens);
    }

    /// Get the tokens associated with a market.
    #[cfg(test)]
    fn get_market_tokens(&self, market_id: &MarketId) -> Option<Vec<TokenId>> {
        let market_tokens = self.market_tokens.read().expect("lock poisoned");
        market_tokens.get(market_id).cloned()
    }
}

#[async_trait]
impl SubscriptionManager for PrioritySubscriptionManager {
    fn enqueue(&self, markets: Vec<MarketScore>) {
        let mut pending = self.pending.write().expect("lock poisoned");
        let active_markets = self.active_markets.read().expect("lock poisoned");

        for market in markets {
            // Skip markets that are already subscribed
            if active_markets.contains(market.market_id()) {
                debug!(
                    market_id = %market.market_id(),
                    "Market already subscribed, skipping enqueue"
                );
                continue;
            }

            debug!(
                market_id = %market.market_id(),
                score = market.composite(),
                "Enqueueing market for subscription"
            );
            pending.push(market);
        }
    }

    fn active_subscriptions(&self) -> Vec<TokenId> {
        let active_tokens = self.active_tokens.read().expect("lock poisoned");
        active_tokens.clone()
    }

    fn active_count(&self) -> usize {
        let active_tokens = self.active_tokens.read().expect("lock poisoned");
        active_tokens.len()
    }

    fn pending_count(&self) -> usize {
        let pending = self.pending.read().expect("lock poisoned");
        pending.len()
    }

    async fn expand(&self, count: usize) -> Result<Vec<TokenId>> {
        let mut pending = self.pending.write().expect("lock poisoned");
        let mut active_markets = self.active_markets.write().expect("lock poisoned");
        let mut active_tokens = self.active_tokens.write().expect("lock poisoned");
        let market_tokens = self.market_tokens.read().expect("lock poisoned");

        let mut newly_subscribed = Vec::new();
        let mut markets_added = 0;

        // Keep popping from the queue until we've added enough tokens or run out
        while markets_added < count {
            let Some(market_score) = pending.pop() else {
                debug!("No more markets in pending queue");
                break;
            };

            let market_id = market_score.market_id();

            // Skip if already active (could happen with duplicates in queue)
            if active_markets.contains(market_id) {
                continue;
            }

            // Get the tokens for this market
            let Some(tokens) = market_tokens.get(market_id) else {
                warn!(
                    market_id = %market_id,
                    "No token mapping found for market, skipping"
                );
                continue;
            };

            // Check if we would exceed max subscriptions
            let new_count = active_tokens.len() + tokens.len();
            if new_count > self.max_subscriptions {
                debug!(
                    market_id = %market_id,
                    current = active_tokens.len(),
                    adding = tokens.len(),
                    max = self.max_subscriptions,
                    "Would exceed max subscriptions, stopping expansion"
                );
                // Push the market back to the queue
                pending.push(market_score);
                break;
            }

            info!(
                market_id = %market_id,
                score = market_score.composite(),
                token_count = tokens.len(),
                "Expanding subscription to market"
            );

            active_markets.insert(market_id.clone());
            for token in tokens {
                active_tokens.push(token.clone());
                newly_subscribed.push(token.clone());
            }
            markets_added += 1;
        }

        Ok(newly_subscribed)
    }

    async fn contract(&self, count: usize) -> Result<Vec<TokenId>> {
        let mut active_tokens = self.active_tokens.write().expect("lock poisoned");
        let mut active_markets = self.active_markets.write().expect("lock poisoned");
        let market_tokens = self.market_tokens.read().expect("lock poisoned");

        let mut removed_tokens = Vec::new();
        let mut tokens_to_remove = count.min(active_tokens.len());

        // LIFO removal: remove from the end of the active tokens list
        while tokens_to_remove > 0 && !active_tokens.is_empty() {
            if let Some(token) = active_tokens.pop() {
                removed_tokens.push(token);
                tokens_to_remove -= 1;
            }
        }

        // Update active_markets by checking which markets no longer have active tokens
        let active_token_set: HashSet<_> = active_tokens.iter().collect();

        // Find markets that no longer have any active tokens
        let markets_to_remove: Vec<_> = active_markets
            .iter()
            .filter(|market_id| {
                if let Some(tokens) = market_tokens.get(*market_id) {
                    !tokens.iter().any(|t| active_token_set.contains(t))
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        for market_id in &markets_to_remove {
            debug!(
                market_id = %market_id,
                "Removing market from active subscriptions"
            );
            active_markets.remove(market_id);
        }

        info!(
            removed_tokens = removed_tokens.len(),
            removed_markets = markets_to_remove.len(),
            "Contracted subscriptions"
        );

        Ok(removed_tokens)
    }

    async fn on_connection_event(&self, event: ConnectionEvent) -> Result<()> {
        match &event {
            ConnectionEvent::Connected { connection_id } => {
                info!(connection_id, "Connection established");
            }
            ConnectionEvent::Disconnected {
                connection_id,
                reason,
            } => {
                warn!(connection_id, reason, "Connection lost");
            }
            ConnectionEvent::ShardUnhealthy { shard_id } => {
                warn!(shard_id, "Shard became unhealthy");
            }
            ConnectionEvent::ShardRecovered { shard_id } => {
                info!(shard_id, "Shard recovered");
            }
        }
        Ok(())
    }

    fn is_subscribed(&self, market_id: &MarketId) -> bool {
        let active_markets = self.active_markets.read().expect("lock poisoned");
        active_markets.contains(market_id)
    }

    fn max_subscriptions(&self) -> usize {
        self.max_subscriptions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::domain::ScoreFactors;

    fn make_market_id(name: &str) -> MarketId {
        MarketId::new(name)
    }

    fn make_token_id(name: &str) -> TokenId {
        TokenId::new(name)
    }

    fn make_market_score(market_id: &str, composite: f64) -> MarketScore {
        let factors = ScoreFactors::default();
        MarketScore::new(make_market_id(market_id), factors, composite)
    }

    // --- Constructor tests ---

    #[test]
    fn new_creates_empty_manager() {
        let manager = PrioritySubscriptionManager::new(100);

        assert_eq!(manager.max_subscriptions(), 100);
        assert_eq!(manager.active_count(), 0);
        assert_eq!(manager.pending_count(), 0);
        assert!(manager.active_subscriptions().is_empty());
    }

    #[test]
    fn new_with_zero_max_subscriptions() {
        let manager = PrioritySubscriptionManager::new(0);
        assert_eq!(manager.max_subscriptions(), 0);
    }

    // --- register_market_tokens tests ---

    #[test]
    fn register_market_tokens_stores_mapping() {
        let manager = PrioritySubscriptionManager::new(100);
        let market_id = make_market_id("market-1");
        let tokens = vec![make_token_id("token-1a"), make_token_id("token-1b")];

        manager.register_market_tokens(market_id.clone(), tokens.clone());

        let retrieved = manager.get_market_tokens(&market_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().len(), 2);
    }

    #[test]
    fn register_market_tokens_overwrites_existing() {
        let manager = PrioritySubscriptionManager::new(100);
        let market_id = make_market_id("market-1");

        manager.register_market_tokens(
            market_id.clone(),
            vec![make_token_id("old-token")],
        );
        manager.register_market_tokens(
            market_id.clone(),
            vec![make_token_id("new-token-1"), make_token_id("new-token-2")],
        );

        let retrieved = manager.get_market_tokens(&market_id).unwrap();
        assert_eq!(retrieved.len(), 2);
        assert_eq!(retrieved[0].as_str(), "new-token-1");
    }

    // --- enqueue tests ---

    #[test]
    fn enqueue_adds_to_pending() {
        let manager = PrioritySubscriptionManager::new(100);
        let score = make_market_score("market-1", 0.5);

        manager.enqueue(vec![score]);

        assert_eq!(manager.pending_count(), 1);
    }

    #[test]
    fn enqueue_multiple_markets() {
        let manager = PrioritySubscriptionManager::new(100);
        let scores = vec![
            make_market_score("market-1", 0.5),
            make_market_score("market-2", 0.7),
            make_market_score("market-3", 0.3),
        ];

        manager.enqueue(scores);

        assert_eq!(manager.pending_count(), 3);
    }

    #[test]
    fn enqueue_skips_already_subscribed() {
        let manager = PrioritySubscriptionManager::new(100);
        let market_id = make_market_id("market-1");
        let tokens = vec![make_token_id("token-1")];

        // Register and subscribe
        manager.register_market_tokens(market_id.clone(), tokens);

        // Manually add to active markets
        {
            let mut active = manager.active_markets.write().unwrap();
            active.insert(market_id.clone());
        }

        // Try to enqueue the same market
        let score = make_market_score("market-1", 0.5);
        manager.enqueue(vec![score]);

        // Should not be added to pending
        assert_eq!(manager.pending_count(), 0);
    }

    // --- expand tests ---

    #[tokio::test]
    async fn expand_adds_highest_priority_first() {
        let manager = PrioritySubscriptionManager::new(100);

        // Register markets with tokens
        manager.register_market_tokens(
            make_market_id("low"),
            vec![make_token_id("low-token")],
        );
        manager.register_market_tokens(
            make_market_id("high"),
            vec![make_token_id("high-token")],
        );
        manager.register_market_tokens(
            make_market_id("medium"),
            vec![make_token_id("medium-token")],
        );

        // Enqueue in random order
        manager.enqueue(vec![
            make_market_score("low", 0.2),
            make_market_score("high", 0.9),
            make_market_score("medium", 0.5),
        ]);

        // Expand by 1 - should get the highest priority market
        let added = manager.expand(1).await.unwrap();

        assert_eq!(added.len(), 1);
        assert_eq!(added[0].as_str(), "high-token");
        assert!(manager.is_subscribed(&make_market_id("high")));
        assert!(!manager.is_subscribed(&make_market_id("medium")));
        assert!(!manager.is_subscribed(&make_market_id("low")));
    }

    #[tokio::test]
    async fn expand_multiple_markets() {
        let manager = PrioritySubscriptionManager::new(100);

        manager.register_market_tokens(
            make_market_id("market-1"),
            vec![make_token_id("token-1")],
        );
        manager.register_market_tokens(
            make_market_id("market-2"),
            vec![make_token_id("token-2")],
        );

        manager.enqueue(vec![
            make_market_score("market-1", 0.5),
            make_market_score("market-2", 0.6),
        ]);

        let added = manager.expand(2).await.unwrap();

        assert_eq!(added.len(), 2);
        assert_eq!(manager.active_count(), 2);
        assert_eq!(manager.pending_count(), 0);
    }

    #[tokio::test]
    async fn expand_respects_max_subscriptions() {
        let manager = PrioritySubscriptionManager::new(2); // Only 2 tokens allowed

        manager.register_market_tokens(
            make_market_id("market-1"),
            vec![make_token_id("token-1a"), make_token_id("token-1b")], // 2 tokens
        );
        manager.register_market_tokens(
            make_market_id("market-2"),
            vec![make_token_id("token-2")],
        );

        manager.enqueue(vec![
            make_market_score("market-1", 0.5),
            make_market_score("market-2", 0.3),
        ]);

        let added = manager.expand(10).await.unwrap();

        // Should only add market-1's tokens (2 tokens = max)
        assert_eq!(added.len(), 2);
        assert_eq!(manager.active_count(), 2);
        assert!(manager.is_subscribed(&make_market_id("market-1")));
        // market-2 should still be pending
        assert_eq!(manager.pending_count(), 1);
    }

    #[tokio::test]
    async fn expand_skips_markets_without_token_mapping() {
        let manager = PrioritySubscriptionManager::new(100);

        // Don't register tokens for market-1
        manager.register_market_tokens(
            make_market_id("market-2"),
            vec![make_token_id("token-2")],
        );

        manager.enqueue(vec![
            make_market_score("market-1", 0.9), // No token mapping
            make_market_score("market-2", 0.5),
        ]);

        let added = manager.expand(2).await.unwrap();

        // Should skip market-1 and add market-2
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].as_str(), "token-2");
    }

    #[tokio::test]
    async fn expand_returns_empty_when_queue_empty() {
        let manager = PrioritySubscriptionManager::new(100);

        let added = manager.expand(5).await.unwrap();

        assert!(added.is_empty());
    }

    // --- contract tests ---

    #[tokio::test]
    async fn contract_removes_tokens_lifo() {
        let manager = PrioritySubscriptionManager::new(100);

        manager.register_market_tokens(
            make_market_id("market-1"),
            vec![make_token_id("token-1")],
        );
        manager.register_market_tokens(
            make_market_id("market-2"),
            vec![make_token_id("token-2")],
        );

        manager.enqueue(vec![
            make_market_score("market-1", 0.5),
            make_market_score("market-2", 0.6),
        ]);

        manager.expand(2).await.unwrap();

        // Contract by 1 - should remove the last added token
        let removed = manager.contract(1).await.unwrap();

        assert_eq!(removed.len(), 1);
        assert_eq!(manager.active_count(), 1);
    }

    #[tokio::test]
    async fn contract_removes_market_when_no_tokens_left() {
        let manager = PrioritySubscriptionManager::new(100);

        manager.register_market_tokens(
            make_market_id("market-1"),
            vec![make_token_id("token-1")],
        );

        manager.enqueue(vec![make_market_score("market-1", 0.5)]);
        manager.expand(1).await.unwrap();

        assert!(manager.is_subscribed(&make_market_id("market-1")));

        manager.contract(1).await.unwrap();

        assert!(!manager.is_subscribed(&make_market_id("market-1")));
        assert_eq!(manager.active_count(), 0);
    }

    #[tokio::test]
    async fn contract_handles_more_than_active() {
        let manager = PrioritySubscriptionManager::new(100);

        manager.register_market_tokens(
            make_market_id("market-1"),
            vec![make_token_id("token-1")],
        );

        manager.enqueue(vec![make_market_score("market-1", 0.5)]);
        manager.expand(1).await.unwrap();

        // Try to contract more than we have
        let removed = manager.contract(10).await.unwrap();

        assert_eq!(removed.len(), 1);
        assert_eq!(manager.active_count(), 0);
    }

    #[tokio::test]
    async fn contract_returns_empty_when_no_active() {
        let manager = PrioritySubscriptionManager::new(100);

        let removed = manager.contract(5).await.unwrap();

        assert!(removed.is_empty());
    }

    // --- is_subscribed tests ---

    #[test]
    fn is_subscribed_returns_false_for_unknown_market() {
        let manager = PrioritySubscriptionManager::new(100);

        assert!(!manager.is_subscribed(&make_market_id("unknown")));
    }

    #[tokio::test]
    async fn is_subscribed_returns_true_after_expand() {
        let manager = PrioritySubscriptionManager::new(100);

        manager.register_market_tokens(
            make_market_id("market-1"),
            vec![make_token_id("token-1")],
        );
        manager.enqueue(vec![make_market_score("market-1", 0.5)]);

        assert!(!manager.is_subscribed(&make_market_id("market-1")));

        manager.expand(1).await.unwrap();

        assert!(manager.is_subscribed(&make_market_id("market-1")));
    }

    // --- active_subscriptions tests ---

    #[tokio::test]
    async fn active_subscriptions_returns_all_tokens() {
        let manager = PrioritySubscriptionManager::new(100);

        manager.register_market_tokens(
            make_market_id("market-1"),
            vec![make_token_id("token-1a"), make_token_id("token-1b")],
        );

        manager.enqueue(vec![make_market_score("market-1", 0.5)]);
        manager.expand(1).await.unwrap();

        let active = manager.active_subscriptions();
        assert_eq!(active.len(), 2);
    }

    // --- on_connection_event tests ---

    #[tokio::test]
    async fn on_connection_event_connected() {
        let manager = PrioritySubscriptionManager::new(100);
        let event = ConnectionEvent::Connected { connection_id: 1 };

        let result = manager.on_connection_event(event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn on_connection_event_disconnected() {
        let manager = PrioritySubscriptionManager::new(100);
        let event = ConnectionEvent::Disconnected {
            connection_id: 1,
            reason: "test".to_string(),
        };

        let result = manager.on_connection_event(event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn on_connection_event_shard_unhealthy() {
        let manager = PrioritySubscriptionManager::new(100);
        let event = ConnectionEvent::ShardUnhealthy { shard_id: 1 };

        let result = manager.on_connection_event(event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn on_connection_event_shard_recovered() {
        let manager = PrioritySubscriptionManager::new(100);
        let event = ConnectionEvent::ShardRecovered { shard_id: 1 };

        let result = manager.on_connection_event(event).await;
        assert!(result.is_ok());
    }

    // --- Thread safety test ---

    #[tokio::test]
    async fn concurrent_operations() {
        use std::sync::Arc;

        let manager = Arc::new(PrioritySubscriptionManager::new(1000));

        // Register some markets
        for i in 0..10 {
            manager.register_market_tokens(
                make_market_id(&format!("market-{i}")),
                vec![make_token_id(&format!("token-{i}"))],
            );
        }

        // Spawn concurrent enqueue tasks
        let mut handles = vec![];
        for i in 0..10 {
            let manager_clone = Arc::clone(&manager);
            handles.push(tokio::spawn(async move {
                let score = make_market_score(&format!("market-{i}"), i as f64 / 10.0);
                manager_clone.enqueue(vec![score]);
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }

        // All should be in pending queue
        assert_eq!(manager.pending_count(), 10);

        // Expand all
        manager.expand(10).await.unwrap();
        assert_eq!(manager.active_count(), 10);
        assert_eq!(manager.pending_count(), 0);
    }
}
