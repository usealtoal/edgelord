//! Inference port for discovering market relations.
//!
//! This module defines the trait for inferring logical relationships
//! between prediction markets using LLMs or other methods.

use async_trait::async_trait;

use crate::domain::{MarketId, Relation};
use crate::error::Result;

/// Market summary for inference (minimal data needed).
///
/// This is a simplified view of a market containing only
/// the information needed for relation inference.
#[derive(Debug, Clone)]
pub struct MarketSummary {
    /// Unique market identifier.
    pub id: MarketId,
    /// The market question (e.g., "Will X happen by Y date?").
    pub question: String,
    /// Outcome names (e.g., ["Yes", "No"] or ["Trump", "Biden", "Other"]).
    pub outcomes: Vec<String>,
}

impl MarketSummary {
    /// Create a new market summary.
    #[must_use]
    pub fn new(id: MarketId, question: impl Into<String>, outcomes: Vec<String>) -> Self {
        Self {
            id,
            question: question.into(),
            outcomes,
        }
    }

    /// Check if this is a binary market.
    #[must_use]
    pub fn is_binary(&self) -> bool {
        self.outcomes.len() == 2
    }
}

/// Infers relations between markets.
///
/// Implementations analyze market questions to discover logical
/// dependencies like:
/// - "A implies B" (if A is true, B must be true)
/// - "Mutually exclusive" (at most one can be true)
/// - "Exactly one" (exactly one must be true)
///
/// # Implementation Notes
///
/// - Implementations must be thread-safe (`Send + Sync`)
/// - The `infer` method is async to support LLM API calls
/// - Respect `batch_limit` to avoid overwhelming the inference backend
#[async_trait]
pub trait RelationInferrer: Send + Sync {
    /// Inferrer name for logging.
    fn name(&self) -> &'static str;

    /// Infer relations from a set of markets.
    ///
    /// # Arguments
    ///
    /// * `markets` - Market summaries to analyze for relations
    ///
    /// # Returns
    ///
    /// Discovered relations with confidence scores.
    /// May return an empty vector if no relations are found.
    async fn infer(&self, markets: &[MarketSummary]) -> Result<Vec<Relation>>;

    /// Maximum markets per inference call.
    ///
    /// Implementations can override this based on their backend's
    /// context window or rate limits.
    fn batch_limit(&self) -> usize {
        30
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// Mock inferrer for testing.
    pub struct MockInferrer {
        relations: Vec<Relation>,
    }

    impl MockInferrer {
        pub fn new(relations: Vec<Relation>) -> Self {
            Self { relations }
        }
    }

    #[async_trait]
    impl RelationInferrer for MockInferrer {
        fn name(&self) -> &'static str {
            "mock"
        }

        async fn infer(&self, _markets: &[MarketSummary]) -> Result<Vec<Relation>> {
            Ok(self.relations.clone())
        }
    }
}
