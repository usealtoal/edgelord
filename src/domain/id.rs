//! Domain identifier types with proper encapsulation.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Token identifier - newtype for type safety.
///
/// The inner String is private to ensure all construction goes through
/// the defined constructors.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TokenId(String);

impl TokenId {
    /// Create a new `TokenId` from a string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the token ID as a string slice.
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

/// Market condition identifier - newtype for type safety.
///
/// The inner String is private to ensure all construction goes through
/// the defined constructors.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MarketId(String);

impl MarketId {
    /// Create a new `MarketId` from a string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the market ID as a string slice.
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
/// Generated as UUID v4 for new relations, or constructed from
/// existing string for persistence/deserialization.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RelationId(String);

impl RelationId {
    /// Create a new `RelationId` with a generated UUID.
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Get the relation ID as a string slice.
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
/// Generated as UUID v4 for new clusters, or constructed from
/// existing string for persistence/deserialization.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClusterId(String);

impl ClusterId {
    /// Create a new `ClusterId` with a generated UUID.
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Get the cluster ID as a string slice.
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

/// Unique identifier for an order.
///
/// The inner String is private to ensure all construction goes through
/// the defined constructors.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrderId(String);

impl OrderId {
    /// Create a new order ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the order ID as a string slice.
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

/// Unique position identifier.
///
/// The inner u64 is private to ensure all construction goes through
/// the defined constructors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PositionId(u64);

impl PositionId {
    /// Create a new `PositionId` from a u64 value.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the underlying value.
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
