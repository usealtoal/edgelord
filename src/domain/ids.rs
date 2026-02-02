//! Domain identifier types with proper encapsulation.

use std::fmt;

/// Token identifier - newtype for type safety.
///
/// The inner String is private to ensure all construction goes through
/// the defined constructors.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TokenId(String);

impl TokenId {
    /// Create a new TokenId from a string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the token ID as a string slice.
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MarketId(String);

impl MarketId {
    /// Create a new MarketId from a string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the market ID as a string slice.
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
}
