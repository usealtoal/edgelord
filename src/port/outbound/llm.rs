//! LLM completion port for inference adapters.

use async_trait::async_trait;

use crate::error::Result;

/// LLM completion client contract.
#[async_trait]
pub trait Llm: Send + Sync {
    /// Provider name for logging.
    fn name(&self) -> &'static str;

    /// Send a completion request and return the response text.
    async fn complete(&self, prompt: &str) -> Result<String>;
}
