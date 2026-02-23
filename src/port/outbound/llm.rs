//! LLM completion port for inference adapters.
//!
//! Defines a generic interface for large language model completion requests.
//! Used by relation inference and other LLM-powered features.

use async_trait::async_trait;

use crate::error::Result;

/// Client for large language model text completion.
///
/// Implementations wrap specific LLM providers (OpenAI, Anthropic, etc.) and
/// handle authentication, rate limiting, and response parsing.
///
/// # Thread Safety
///
/// Implementations must be thread-safe (`Send + Sync`) to support concurrent
/// inference requests.
///
/// # Errors
///
/// The [`complete`](Self::complete) method returns an error for API failures,
/// rate limits, or invalid responses.
#[async_trait]
pub trait Llm: Send + Sync {
    /// Return the provider name for logging and metrics.
    fn name(&self) -> &'static str;

    /// Send a completion request and return the generated text.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The input prompt to complete.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails, the response is invalid,
    /// or rate limits are exceeded.
    async fn complete(&self, prompt: &str) -> Result<String>;
}
