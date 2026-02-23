//! Domain identifier types with proper encapsulation.
//!
//! This module provides strongly-typed identifiers for domain entities.
//! Using newtypes instead of raw strings prevents accidental mixing of
//! different identifier types and provides better documentation.
//!
//! # Type Safety
//!
//! Each identifier type wraps a string (or integer) and only exposes
//! controlled construction methods. This ensures:
//!
//! - Compile-time prevention of mixing market IDs with token IDs
//! - Clear documentation of what each function expects
//! - Easy refactoring if identifier formats change
//!
//! # Examples
//!
//! ```
//! use edgelord::domain::id::{TokenId, MarketId, PositionId};
//!
//! let token = TokenId::new("0x1234abcd");
//! let market = MarketId::new("election-2024-president");
//! let position = PositionId::new(42);
//!
//! // Type safety prevents mixing:
//! // let wrong: TokenId = market; // Compile error!
//! ```

use std::fmt;

use serde::{Deserialize, Serialize};

/// Unique identifier for a tradeable token (outcome share).
///
/// Each outcome in a prediction market has a unique token ID used for
/// trading operations. The inner string is private to ensure all
/// construction goes through defined constructors.
///
/// # Examples
///
/// ```
/// use edgelord::domain::id::TokenId;
///
/// let token = TokenId::new("yes-token-12345");
/// assert_eq!(token.as_str(), "yes-token-12345");
///
/// // From string conversion
/// let token2 = TokenId::from("no-token-67890");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TokenId(String);

impl TokenId {
    /// Creates a new token identifier from a string value.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the token ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TokenId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for TokenId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for TokenId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// Unique identifier for a prediction market.
///
/// Markets are the top-level entity containing one or more outcomes.
/// The inner string is private to ensure all construction goes through
/// defined constructors.
///
/// # Examples
///
/// ```
/// use edgelord::domain::id::MarketId;
///
/// let market = MarketId::new("polymarket-election-2024");
/// assert_eq!(market.as_str(), "polymarket-election-2024");
///
/// // Display formatting
/// println!("Market: {}", market);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MarketId(String);

impl MarketId {
    /// Creates a new market identifier from a string value.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the market ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MarketId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for MarketId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for MarketId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// Unique identifier for an inferred relation between markets.
///
/// Generated as UUID v4 for new relations. For persistence and deserialization,
/// use the `From<String>` implementation to reconstruct from stored values.
///
/// # Examples
///
/// ```
/// use edgelord::domain::id::RelationId;
///
/// // Generate a new unique ID
/// let id1 = RelationId::new();
/// let id2 = RelationId::new();
/// assert_ne!(id1, id2);
///
/// // Reconstruct from stored value
/// let stored = RelationId::from("550e8400-e29b-41d4-a716-446655440000");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RelationId(String);

impl RelationId {
    /// Creates a new relation identifier with a randomly generated UUID v4.
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Returns the relation ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for RelationId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RelationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for RelationId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for RelationId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Unique identifier for a cluster of related markets.
///
/// Clusters group markets connected by logical relations. Generated as UUID v4
/// for new clusters. For persistence and deserialization, use the `From<String>`
/// implementation to reconstruct from stored values.
///
/// # Examples
///
/// ```
/// use edgelord::domain::id::ClusterId;
///
/// // Generate a new unique ID
/// let cluster = ClusterId::new();
/// println!("Created cluster: {}", cluster);
///
/// // Default also generates a new ID
/// let cluster2 = ClusterId::default();
/// assert_ne!(cluster, cluster2);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClusterId(String);

impl ClusterId {
    /// Creates a new cluster identifier with a randomly generated UUID v4.
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Returns the cluster ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ClusterId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ClusterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ClusterId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ClusterId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Unique identifier for an exchange order.
///
/// Order IDs are typically assigned by the exchange when an order is placed.
/// The inner string is private to ensure all construction goes through
/// defined constructors.
///
/// # Examples
///
/// ```
/// use edgelord::domain::id::OrderId;
///
/// let order = OrderId::new("exchange-order-12345");
/// assert_eq!(order.as_str(), "exchange-order-12345");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrderId(String);

impl OrderId {
    /// Creates a new order identifier from a string value.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the order ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for OrderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for OrderId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for OrderId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// Unique identifier for a trading position.
///
/// Positions track open arbitrage trades with their entry costs and expected
/// payouts. The inner `u64` is private to ensure all construction goes through
/// defined constructors.
///
/// # Examples
///
/// ```
/// use edgelord::domain::id::PositionId;
///
/// let pos = PositionId::new(42);
/// assert_eq!(pos.value(), 42);
///
/// // Display shows "pos-N" format
/// assert_eq!(format!("{}", pos), "pos-42");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PositionId(u64);

impl PositionId {
    /// Creates a new position identifier from a numeric value.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Returns the underlying numeric value.
    #[must_use]
    pub const fn value(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for PositionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "pos-{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_id_new_and_as_str() {
        let id = TokenId::new("test-token");
        assert_eq!(id.as_str(), "test-token");
    }

    #[test]
    fn token_id_from_string() {
        let id = TokenId::from("hello".to_string());
        assert_eq!(id.as_str(), "hello");
    }

    #[test]
    fn token_id_from_str() {
        let id = TokenId::from("world");
        assert_eq!(id.as_str(), "world");
    }

    #[test]
    fn token_id_display() {
        let id = TokenId::new("display-test");
        assert_eq!(format!("{}", id), "display-test");
    }

    #[test]
    fn market_id_new_and_as_str() {
        let id = MarketId::new("test-market");
        assert_eq!(id.as_str(), "test-market");
    }

    #[test]
    fn market_id_from_string() {
        let id = MarketId::from("hello".to_string());
        assert_eq!(id.as_str(), "hello");
    }

    #[test]
    fn market_id_from_str() {
        let id = MarketId::from("world");
        assert_eq!(id.as_str(), "world");
    }

    #[test]
    fn market_id_display() {
        let id = MarketId::new("display-test");
        assert_eq!(format!("{}", id), "display-test");
    }

    // RelationId tests
    #[test]
    fn relation_id_generates_unique_ids() {
        let id1 = RelationId::new();
        let id2 = RelationId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn relation_id_as_str_returns_uuid_format() {
        let id = RelationId::new();
        // UUID v4 format: 8-4-4-4-12 hex chars
        assert_eq!(id.as_str().len(), 36);
        assert!(id.as_str().chars().filter(|c| *c == '-').count() == 4);
    }

    #[test]
    fn relation_id_from_string() {
        let id = RelationId::from("existing-id".to_string());
        assert_eq!(id.as_str(), "existing-id");
    }

    #[test]
    fn relation_id_display() {
        let id = RelationId::from("display-test".to_string());
        assert_eq!(format!("{}", id), "display-test");
    }

    #[test]
    fn relation_id_default_generates_new() {
        let id1 = RelationId::default();
        let id2 = RelationId::default();
        assert_ne!(id1, id2);
    }

    // ClusterId tests
    #[test]
    fn cluster_id_generates_unique_ids() {
        let id1 = ClusterId::new();
        let id2 = ClusterId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn cluster_id_as_str_returns_uuid_format() {
        let id = ClusterId::new();
        assert_eq!(id.as_str().len(), 36);
        assert!(id.as_str().chars().filter(|c| *c == '-').count() == 4);
    }

    #[test]
    fn cluster_id_from_string() {
        let id = ClusterId::from("existing-cluster".to_string());
        assert_eq!(id.as_str(), "existing-cluster");
    }

    #[test]
    fn cluster_id_display() {
        let id = ClusterId::from("cluster-display".to_string());
        assert_eq!(format!("{}", id), "cluster-display");
    }

    #[test]
    fn cluster_id_default_generates_new() {
        let id1 = ClusterId::default();
        let id2 = ClusterId::default();
        assert_ne!(id1, id2);
    }

    // OrderId tests
    #[test]
    fn order_id_new_and_as_str() {
        let id = OrderId::new("order-123");
        assert_eq!(id.as_str(), "order-123");
    }

    #[test]
    fn order_id_from_string() {
        let id = OrderId::from("order-456".to_string());
        assert_eq!(id.as_str(), "order-456");
    }

    #[test]
    fn order_id_from_str() {
        let id = OrderId::from("order-789");
        assert_eq!(id.as_str(), "order-789");
    }

    #[test]
    fn order_id_display() {
        let id = OrderId::new("order-display");
        assert_eq!(format!("{}", id), "order-display");
    }

    // PositionId tests
    #[test]
    fn position_id_new_and_value() {
        let id = PositionId::new(42);
        assert_eq!(id.value(), 42);
    }

    #[test]
    fn position_id_display() {
        let id = PositionId::new(123);
        assert_eq!(format!("{}", id), "pos-123");
    }
}
