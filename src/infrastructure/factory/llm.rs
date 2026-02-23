//! LLM client factory.
//!
//! Provides factory functions for constructing LLM clients based on
//! configuration. Supports both Anthropic and OpenAI providers.

use std::sync::Arc;

use tracing::{info, warn};

use crate::adapter::outbound::llm::anthropic::Anthropic;
use crate::adapter::outbound::llm::openai::OpenAi;
use crate::infrastructure::config::llm::LlmProvider;
use crate::infrastructure::config::settings::Config;
use crate::port::outbound::llm::Llm;

/// Build an LLM client from configuration.
///
/// Creates the appropriate client based on the configured provider.
/// Returns `None` if inference is disabled or the required API key
/// environment variable is not set.
///
/// # Environment Variables
///
/// - `ANTHROPIC_API_KEY` - Required for Anthropic provider
/// - `OPENAI_API_KEY` - Required for OpenAI provider
pub fn build_llm_client(config: &Config) -> Option<Arc<dyn Llm>> {
    if !config.inference.enabled {
        return None;
    }

    let client: Arc<dyn Llm> = match config.llm.provider {
        LlmProvider::Anthropic => {
            let api_key = match std::env::var("ANTHROPIC_API_KEY") {
                Ok(key) => key,
                Err(_) => {
                    warn!("ANTHROPIC_API_KEY not set, inference disabled");
                    return None;
                }
            };
            Arc::new(Anthropic::new(
                api_key,
                &config.llm.anthropic.model,
                config.llm.anthropic.max_tokens,
                config.llm.anthropic.temperature,
            ))
        }
        LlmProvider::OpenAi => {
            let api_key = match std::env::var("OPENAI_API_KEY") {
                Ok(key) => key,
                Err(_) => {
                    warn!("OPENAI_API_KEY not set, inference disabled");
                    return None;
                }
            };
            Arc::new(OpenAi::new(
                api_key,
                &config.llm.openai.model,
                config.llm.openai.max_tokens,
                config.llm.openai.temperature,
            ))
        }
    };

    info!(provider = client.name(), "LLM client initialized");
    Some(client)
}
