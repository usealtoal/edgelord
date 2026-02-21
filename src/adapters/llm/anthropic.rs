//! Anthropic Claude LLM client.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::Llm;
use crate::error::{Error, Result};

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";

/// Anthropic Claude client.
pub struct AnthropicLlm {
    client: Client,
    api_key: String,
    model: String,
    max_tokens: usize,
    temperature: f64,
}

impl AnthropicLlm {
    /// Create a new Anthropic client.
    pub fn new(
        api_key: impl Into<String>,
        model: impl Into<String>,
        max_tokens: usize,
        temperature: f64,
    ) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
            model: model.into(),
            max_tokens,
            temperature,
        }
    }

    /// Create from environment variable.
    pub fn from_env(model: impl Into<String>) -> Result<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| {
            Error::Config(crate::error::ConfigError::MissingField {
                field: "ANTHROPIC_API_KEY",
            })
        })?;
        Ok(Self::new(api_key, model, 4096, 0.2))
    }
}

#[derive(Serialize)]
struct Request {
    model: String,
    max_tokens: usize,
    temperature: f64,
    messages: Vec<Message>,
}

#[derive(Serialize)]
struct Message {
    role: &'static str,
    content: String,
}

#[derive(Deserialize)]
struct Response {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    text: String,
}

#[async_trait]
impl Llm for AnthropicLlm {
    fn name(&self) -> &'static str {
        "anthropic"
    }

    async fn complete(&self, prompt: &str) -> Result<String> {
        let request = Request {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            messages: vec![Message {
                role: "user",
                content: prompt.to_string(),
            }],
        };

        let response = self
            .client
            .post(API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| Error::Connection(e.to_string()))?
            .json::<Response>()
            .await?;

        Ok(response
            .content
            .into_iter()
            .map(|c| c.text)
            .collect::<Vec<_>>()
            .join(""))
    }
}
