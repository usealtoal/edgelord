//! LLM provider configuration.
//!
//! Provides configuration for Large Language Model providers used for
//! market relationship inference.

use serde::Deserialize;

/// LLM provider configuration.
///
/// Configures which LLM provider to use and provider-specific settings.
/// API keys are read from environment variables (`ANTHROPIC_API_KEY` or
/// `OPENAI_API_KEY`) at runtime.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct LlmConfig {
    /// LLM provider to use for inference.
    ///
    /// Defaults to OpenAI.
    #[serde(default)]
    pub provider: LlmProvider,

    /// Anthropic-specific settings.
    ///
    /// Used when `provider` is set to `anthropic`.
    #[serde(default)]
    pub anthropic: AnthropicConfig,

    /// OpenAI-specific settings.
    ///
    /// Used when `provider` is set to `openai`.
    #[serde(default)]
    pub openai: OpenAiConfig,
}

/// LLM provider selection.
///
/// Determines which LLM API to use for market relationship inference.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    /// Anthropic Claude models.
    Anthropic,
    /// OpenAI GPT models.
    #[default]
    OpenAi,
}

/// Anthropic-specific configuration.
///
/// Settings for Anthropic Claude API. Requires `ANTHROPIC_API_KEY`
/// environment variable.
#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicConfig {
    /// Model identifier.
    ///
    /// Defaults to "claude-3-5-sonnet-20241022".
    #[serde(default = "default_anthropic_model")]
    pub model: String,

    /// Sampling temperature for generation.
    ///
    /// Lower values produce more deterministic output.
    /// Defaults to 0.2.
    #[serde(default = "default_temperature")]
    pub temperature: f64,

    /// Maximum tokens in the response.
    ///
    /// Limits response length to control costs. Defaults to 4096.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            model: default_anthropic_model(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
        }
    }
}

/// OpenAI-specific configuration.
///
/// Settings for OpenAI API. Requires `OPENAI_API_KEY` environment variable.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiConfig {
    /// Model identifier.
    ///
    /// Defaults to "gpt-4o".
    #[serde(default = "default_openai_model")]
    pub model: String,

    /// Sampling temperature for generation.
    ///
    /// Lower values produce more deterministic output.
    /// Defaults to 0.2.
    #[serde(default = "default_temperature")]
    pub temperature: f64,

    /// Maximum tokens in the response.
    ///
    /// Limits response length to control costs. Defaults to 4096.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
}

impl Default for OpenAiConfig {
    fn default() -> Self {
        Self {
            model: default_openai_model(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
        }
    }
}

fn default_anthropic_model() -> String {
    "claude-3-5-sonnet-20241022".into()
}

fn default_openai_model() -> String {
    "gpt-4o".into()
}

fn default_temperature() -> f64 {
    0.2
}

const fn default_max_tokens() -> usize {
    4096
}
