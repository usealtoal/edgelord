//! Relation inference service for discovering market dependencies.

mod llm;

pub use llm::LlmInferrer;

use async_trait::async_trait;

use crate::core::domain::{MarketId, Relation};
use crate::error::Result;

/// Market summary for inference (no exchange dependency).
#[derive(Debug, Clone)]
pub struct MarketSummary {
    pub id: MarketId,
    pub question: String,
    pub outcomes: Vec<String>,
}

/// Infers relations between markets.
#[async_trait]
pub trait Inferrer: Send + Sync {
    /// Inferrer name for logging.
    fn name(&self) -> &'static str;

    /// Infer relations from a set of markets.
    async fn infer(&self, markets: &[MarketSummary]) -> Result<Vec<Relation>>;

    /// Maximum markets per inference call.
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
    impl Inferrer for MockInferrer {
        fn name(&self) -> &'static str {
            "mock"
        }

        async fn infer(&self, _markets: &[MarketSummary]) -> Result<Vec<Relation>> {
            Ok(self.relations.clone())
        }
    }
}
