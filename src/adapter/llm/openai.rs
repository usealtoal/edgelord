//! OpenAI LLM client.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::Llm;
use crate::error::{Error, Result};

const API_URL: &str = "https://api.openai.com/v1/chat/completions";

/// OpenAI client.
pub struct OpenAi {
    client: Client,
    api_key: String,
    model: String,
    max_tokens: usize,
    temperature: f64,
}

impl OpenAi {
    /// Create a new OpenAI client.
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
        let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| {
            Error::Config(crate::error::ConfigError::MissingField {
                field: "OPENAI_API_KEY",
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
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Deserialize)]
struct ChoiceMessage {
    content: String,
}

#[async_trait]
impl Llm for OpenAi {
    fn name(&self) -> &'static str {
        "openai"
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
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| Error::Connection(e.to_string()))?
            .json::<Response>()
            .await?;

        Ok(response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_default())
    }
}
