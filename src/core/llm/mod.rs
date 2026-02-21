//! LLM client abstraction for inference.
//!
//! This module re-exports from `crate::adapters::llm` for backward compatibility.

pub use crate::adapters::llm::{AnthropicLlm, Llm, OpenAiLlm};

#[cfg(test)]
pub use crate::adapters::llm::tests;
