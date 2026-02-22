//! Priority-based subscription manager implementation.
//!
//! This module provides [`PrioritySubscriptionManager`], which manages market subscriptions
//! using a priority queue based on market scores. Higher-scoring markets are given
//! priority when expanding subscriptions.

use std::collections::{BinaryHeap, HashMap, HashSet};
use std::sync::RwLock;

use async_trait::async_trait;

use super::manager::{ConnectionEvent, SubscriptionManager};
use crate::domain::{id::MarketId, id::TokenId, score::MarketScore};
use crate::error::Result;

mod contract;
mod event;
mod queue;
mod state;

use state::write_lock_or_recover;

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
/// use edgelord::infrastructure::subscription::priority::PrioritySubscriptionManager;
/// use edgelord::infrastructure::subscription::manager::SubscriptionManager;
/// use edgelord::domain::{id::MarketId, score::MarketScore, score::ScoreFactors, id::TokenId};
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
    /// use edgelord::infrastructure::subscription::priority::PrioritySubscriptionManager;
    /// use edgelord::infrastructure::subscription::manager::SubscriptionManager;
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
    /// use edgelord::infrastructure::subscription::priority::PrioritySubscriptionManager;
    /// use edgelord::domain::{id::MarketId, id::TokenId};
    ///
    /// let manager = PrioritySubscriptionManager::new(100);
    /// let market_id = MarketId::new("market-1");
    /// let tokens = vec![TokenId::new("token-1a"), TokenId::new("token-1b")];
    ///
    /// manager.register_market_tokens(market_id.clone(), tokens);
    /// ```
    pub fn register_market_tokens(&self, market_id: MarketId, tokens: Vec<TokenId>) {
        let mut market_tokens = write_lock_or_recover(&self.market_tokens);
        market_tokens.insert(market_id, tokens);
    }

    /// Get the tokens associated with a market.
    #[cfg(test)]
    fn get_market_tokens(&self, market_id: &MarketId) -> Option<Vec<TokenId>> {
        let market_tokens = state::read_lock_or_recover(&self.market_tokens);
        market_tokens.get(market_id).cloned()
    }
}

#[async_trait]
impl SubscriptionManager for PrioritySubscriptionManager {
    fn enqueue(&self, markets: Vec<MarketScore>) {
        self.enqueue_markets(markets);
    }

    fn active_subscriptions(&self) -> Vec<TokenId> {
        self.active_tokens_snapshot()
    }

    fn active_count(&self) -> usize {
        self.active_tokens_count()
    }

    fn pending_count(&self) -> usize {
        self.pending_markets_count()
    }

    async fn expand(&self, count: usize) -> Result<Vec<TokenId>> {
        self.expand_markets(count).await
    }

    async fn contract(&self, count: usize) -> Result<Vec<TokenId>> {
        self.contract_tokens(count).await
    }

    async fn on_connection_event(&self, event: ConnectionEvent) -> Result<()> {
        self.log_connection_event(&event);
        Ok(())
    }

    fn is_subscribed(&self, market_id: &MarketId) -> bool {
        self.is_market_subscribed(market_id)
    }

    fn max_subscriptions(&self) -> usize {
        self.max_subscriptions
    }
}

#[cfg(test)]
mod tests;
