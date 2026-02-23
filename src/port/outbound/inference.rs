//! Inference port for discovering market relations.
//!
//! Defines traits for inferring logical relationships between prediction
//! markets using LLMs or other semantic analysis methods.
//!
//! # Overview
//!
//! - [`RelationInferrer`]: Discovers logical relations between markets
//! - [`MarketSummary`]: Minimal market data for inference
//!
//! # Relation Types
//!
//! The inferrer discovers relations such as:
//!
//! - **Implication**: If A is true, B must be true
//! - **Mutual exclusion**: At most one can be true
//! - **Exactly one**: Exactly one must be true (partition)

use async_trait::async_trait;

use crate::domain::{id::MarketId, relation::Relation};
use crate::error::Result;

/// Minimal market information for relation inference.
///
/// Contains only the fields needed to analyze logical relationships
/// between markets.
#[derive(Debug, Clone)]
pub struct MarketSummary {
    /// Unique market identifier.
    pub id: MarketId,

    /// Human-readable market question (e.g., "Will X happen by Y date?").
    pub question: String,

    /// Outcome names (e.g., ["Yes", "No"] or ["Trump", "Biden", "Other"]).
    pub outcomes: Vec<String>,
}

impl MarketSummary {
    /// Create a new market summary.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique market identifier.
    /// * `question` - Human-readable market question.
    /// * `outcomes` - Names of possible outcomes.
    #[must_use]
    pub fn new(id: MarketId, question: impl Into<String>, outcomes: Vec<String>) -> Self {
        Self {
            id,
            question: question.into(),
            outcomes,
        }
    }

    /// Return `true` if this is a binary (two-outcome) market.
    #[must_use]
    pub fn is_binary(&self) -> bool {
        self.outcomes.len() == 2
    }
}

/// Inferrer for discovering logical relations between markets.
///
/// Implementations analyze market questions to discover dependencies that
/// enable cross-market arbitrage detection.
///
/// # Thread Safety
///
/// Implementations must be thread-safe (`Send + Sync`).
///
/// # Implementation Notes
///
/// - The [`infer`](Self::infer) method is async to support LLM API calls
/// - Respect [`batch_limit`](Self::batch_limit) to avoid overwhelming the backend
/// - Return empty vectors when no relations are discovered
#[async_trait]
pub trait RelationInferrer: Send + Sync {
    /// Return the inferrer name for logging and metrics.
    fn name(&self) -> &'static str;

    /// Infer relations from a set of markets.
    ///
    /// # Arguments
    ///
    /// * `markets` - Market summaries to analyze for logical relations.
    ///
    /// Returns discovered relations with confidence scores. May return an
    /// empty vector if no relations are found.
    ///
    /// # Errors
    ///
    /// Returns an error if the inference backend fails (e.g., API error,
    /// rate limit, invalid response).
    async fn infer(&self, markets: &[MarketSummary]) -> Result<Vec<Relation>>;

    /// Return the maximum number of markets per inference call.
    ///
    /// Implementations can override this based on their backend's context
    /// window size or rate limits. Default is 30.
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
