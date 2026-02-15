//! LLM provider configuration.

use serde::Deserialize;

/// LLM configuration.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct LlmConfig {
    /// Which provider to use.
    #[serde(default)]
    pub provider: LlmProvider,
    /// Anthropic-specific settings.
    #[serde(default)]
    pub anthropic: AnthropicConfig,
    /// OpenAI-specific settings.
    #[serde(default)]
    pub openai: OpenAiConfig,
}

/// LLM provider selection.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    Anthropic,
    #[default]
    OpenAi,
}

/// Anthropic-specific configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicConfig {
    /// Model name.
    #[serde(default = "default_anthropic_model")]
    pub model: String,
    /// Temperature for generation.
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    /// Maximum tokens in response.
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
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiConfig {
    /// Model name.
    #[serde(default = "default_openai_model")]
    pub model: String,
    /// Temperature for generation.
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    /// Maximum tokens in response.
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
