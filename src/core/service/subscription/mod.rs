//! Subscription management for adaptive market monitoring.
//!
//! The [`SubscriptionManager`] trait defines the interface for managing WebSocket
//! subscriptions to market data streams. It provides methods for priority-based
//! subscription queuing, dynamic scaling (expand/contract), and connection event handling.
//!
//! # Overview
//!
//! The subscription manager maintains two key collections:
//! - **Active subscriptions**: Markets currently being monitored via WebSocket
//! - **Priority queue**: Markets waiting to be subscribed, ordered by score
//!
//! The manager responds to connection events (disconnects, shard health changes)
//! and supports dynamic scaling based on system resource availability.

mod priority;
pub use priority::PrioritySubscriptionManager;

use async_trait::async_trait;

use crate::core::domain::{MarketId, MarketScore, TokenId};
use crate::error::Result;

/// Events related to WebSocket connection state changes.
///
/// These events are used to notify the subscription manager of connection
/// lifecycle changes, allowing it to react appropriately (e.g., resubscribe
/// after reconnection, redistribute subscriptions after shard failure).
///
/// # Example
///
/// ```
/// use edgelord::core::service::ConnectionEvent;
///
/// let connected = ConnectionEvent::Connected { connection_id: 0 };
/// let disconnected = ConnectionEvent::Disconnected {
///     connection_id: 0,
///     reason: "Server closed connection".to_string(),
/// };
/// let unhealthy = ConnectionEvent::ShardUnhealthy { shard_id: 1 };
/// let recovered = ConnectionEvent::ShardRecovered { shard_id: 1 };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionEvent {
    /// A WebSocket connection was established.
    Connected {
        /// Unique identifier for the connection.
        connection_id: usize,
    },
    /// A WebSocket connection was lost.
    Disconnected {
        /// Unique identifier for the connection that was lost.
        connection_id: usize,
        /// Human-readable reason for the disconnection.
        reason: String,
    },
    /// A shard became unhealthy (high latency, errors, etc.).
    ShardUnhealthy {
        /// Identifier of the unhealthy shard.
        shard_id: usize,
    },
    /// A previously unhealthy shard has recovered.
    ShardRecovered {
        /// Identifier of the recovered shard.
        shard_id: usize,
    },
}

/// Manages subscription lifecycle and priority for market data streams.
///
/// Implementations maintain a priority queue of markets to subscribe to,
/// manage active subscriptions, and respond to connection state changes.
/// The manager supports dynamic scaling through `expand` and `contract` methods.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` to allow sharing across async tasks.
///
/// # Example
///
/// ```ignore
/// use edgelord::core::service::{SubscriptionManager, ConnectionEvent};
/// use edgelord::core::domain::{MarketScore, MarketId, TokenId};
///
/// struct MySubscriptionManager {
///     // ... internal state
/// }
///
/// #[async_trait]
/// impl SubscriptionManager for MySubscriptionManager {
///     fn enqueue(&self, markets: Vec<MarketScore>) {
///         // Add markets to priority queue
///     }
///
///     fn active_subscriptions(&self) -> Vec<TokenId> {
///         // Return currently subscribed tokens
///         vec![]
///     }
///
///     fn active_count(&self) -> usize {
///         0
///     }
///
///     fn pending_count(&self) -> usize {
///         0
///     }
///
///     async fn expand(&self, count: usize) -> Result<Vec<TokenId>> {
///         // Subscribe to `count` more markets from queue
///         Ok(vec![])
///     }
///
///     async fn contract(&self, count: usize) -> Result<Vec<TokenId>> {
///         // Unsubscribe from `count` lowest-priority markets
///         Ok(vec![])
///     }
///
///     async fn on_connection_event(&self, event: ConnectionEvent) -> Result<()> {
///         // Handle connection state changes
///         Ok(())
///     }
///
///     fn is_subscribed(&self, market_id: &MarketId) -> bool {
///         false
///     }
///
///     fn max_subscriptions(&self) -> usize {
///         1000
///     }
/// }
/// ```
#[async_trait]
pub trait SubscriptionManager: Send + Sync {
    /// Add markets to the priority queue for subscription.
    ///
    /// Markets are queued based on their score and will be subscribed
    /// when capacity is available. If a market is already subscribed or
    /// queued, its score may be updated.
    ///
    /// # Arguments
    ///
    /// * `markets` - Markets with their computed scores to enqueue
    fn enqueue(&self, markets: Vec<MarketScore>);

    /// Get the list of currently active subscription tokens.
    ///
    /// Returns all tokens that are currently being monitored via WebSocket.
    ///
    /// # Returns
    ///
    /// Vector of [`TokenId`]s for all active subscriptions.
    fn active_subscriptions(&self) -> Vec<TokenId>;

    /// Get the count of active subscriptions.
    ///
    /// This is equivalent to `active_subscriptions().len()` but may be
    /// more efficient for implementations that track the count separately.
    ///
    /// # Returns
    ///
    /// Number of currently active subscriptions.
    fn active_count(&self) -> usize;

    /// Get the count of markets waiting in the priority queue.
    ///
    /// # Returns
    ///
    /// Number of markets queued but not yet subscribed.
    fn pending_count(&self) -> usize;

    /// Expand subscriptions by subscribing to more markets.
    ///
    /// Takes the highest-priority markets from the queue and subscribes
    /// to them, up to the requested count or available queue depth.
    ///
    /// # Arguments
    ///
    /// * `count` - Maximum number of new subscriptions to add
    ///
    /// # Returns
    ///
    /// Vector of [`TokenId`]s that were successfully subscribed.
    ///
    /// # Errors
    ///
    /// Returns an error if subscription operations fail (e.g., WebSocket errors).
    async fn expand(&self, count: usize) -> Result<Vec<TokenId>>;

    /// Contract subscriptions by unsubscribing from lowest-priority markets.
    ///
    /// Removes the lowest-priority active subscriptions, returning them
    /// to the queue for potential future subscription.
    ///
    /// # Arguments
    ///
    /// * `count` - Number of subscriptions to remove
    ///
    /// # Returns
    ///
    /// Vector of [`TokenId`]s that were unsubscribed.
    ///
    /// # Errors
    ///
    /// Returns an error if unsubscription operations fail.
    async fn contract(&self, count: usize) -> Result<Vec<TokenId>>;

    /// Handle a connection state change event.
    ///
    /// Called when WebSocket connections change state. Implementations
    /// should handle reconnection logic, subscription recovery, and
    /// load redistribution as needed.
    ///
    /// # Arguments
    ///
    /// * `event` - The connection event to handle
    ///
    /// # Errors
    ///
    /// Returns an error if event handling fails.
    async fn on_connection_event(&self, event: ConnectionEvent) -> Result<()>;

    /// Check if a market is currently subscribed.
    ///
    /// # Arguments
    ///
    /// * `market_id` - The market identifier to check
    ///
    /// # Returns
    ///
    /// `true` if the market has an active subscription, `false` otherwise.
    fn is_subscribed(&self, market_id: &MarketId) -> bool;

    /// Get the maximum number of allowed subscriptions.
    ///
    /// This limit may be based on exchange constraints, resource budgets,
    /// or configuration settings.
    ///
    /// # Returns
    ///
    /// Maximum number of subscriptions this manager can maintain.
    fn max_subscriptions(&self) -> usize;
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- ConnectionEvent tests ---

    #[test]
    fn connection_event_connected_debug() {
        let event = ConnectionEvent::Connected { connection_id: 42 };
        let debug = format!("{event:?}");
        assert!(debug.contains("Connected"));
        assert!(debug.contains("42"));
    }

    #[test]
    fn connection_event_disconnected_debug() {
        let event = ConnectionEvent::Disconnected {
            connection_id: 1,
            reason: "timeout".to_string(),
        };
        let debug = format!("{event:?}");
        assert!(debug.contains("Disconnected"));
        assert!(debug.contains("timeout"));
    }

    #[test]
    fn connection_event_shard_unhealthy_debug() {
        let event = ConnectionEvent::ShardUnhealthy { shard_id: 3 };
        let debug = format!("{event:?}");
        assert!(debug.contains("ShardUnhealthy"));
        assert!(debug.contains("3"));
    }

    #[test]
    fn connection_event_shard_recovered_debug() {
        let event = ConnectionEvent::ShardRecovered { shard_id: 5 };
        let debug = format!("{event:?}");
        assert!(debug.contains("ShardRecovered"));
        assert!(debug.contains("5"));
    }

    #[test]
    fn connection_event_clone() {
        let event = ConnectionEvent::Disconnected {
            connection_id: 7,
            reason: "server restart".to_string(),
        };
        let cloned = event.clone();
        assert_eq!(event, cloned);
    }

    #[test]
    fn connection_event_equality() {
        let a = ConnectionEvent::Connected { connection_id: 1 };
        let b = ConnectionEvent::Connected { connection_id: 1 };
        let c = ConnectionEvent::Connected { connection_id: 2 };

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn connection_event_variants_not_equal() {
        let connected = ConnectionEvent::Connected { connection_id: 1 };
        let disconnected = ConnectionEvent::Disconnected {
            connection_id: 1,
            reason: "test".to_string(),
        };
        let unhealthy = ConnectionEvent::ShardUnhealthy { shard_id: 1 };
        let recovered = ConnectionEvent::ShardRecovered { shard_id: 1 };

        assert_ne!(connected, disconnected);
        assert_ne!(unhealthy, recovered);
    }
}
