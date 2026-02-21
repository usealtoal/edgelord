//! Relation inference service for discovering market dependencies.
//!
//! This module re-exports from `crate::ports::inference` for backward compatibility.
//! Implementations are in `crate::adapters::inference`.

mod llm;

pub use llm::LlmInferrer;

// Re-export from ports for backward compatibility
pub use crate::ports::{MarketSummary, RelationInferrer};

// Alias trait for backward compatibility - tests can use `Inferrer` as the trait name
pub use RelationInferrer as Inferrer;
