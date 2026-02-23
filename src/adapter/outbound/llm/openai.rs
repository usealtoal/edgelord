//! OpenAI LLM client.
//!
//! Provides an implementation of the [`Llm`] trait for the OpenAI
//! Chat Completions API.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::port::outbound::llm::Llm;

/// OpenAI Chat Completions API endpoint.
const API_URL: &str = "https://api.openai.com/v1/chat/completions";

/// OpenAI API client.
///
/// Implements the [`Llm`] trait for making chat completion requests
/// to the OpenAI API.
#[derive(Debug)]
pub struct OpenAi {
    /// HTTP client for API requests.
    client: Client,
    /// API key for authentication.
    api_key: String,
    /// Model identifier (e.g., "gpt-4", "gpt-3.5-turbo").
    model: String,
    /// Maximum tokens to generate in the response.
    max_tokens: usize,
    /// Sampling temperature (0.0 to 2.0).
    temperature: f64,
}

impl OpenAi {
    /// Create a new OpenAI client with explicit configuration.
    #[must_use]
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

    /// Create a client from the `OPENAI_API_KEY` environment variable.
    ///
    /// # Errors
    ///
    /// Returns an error if the environment variable is not set.
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

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Request/Response Serialization Tests ====================

    #[test]
    fn test_request_serialization() {
        let request = Request {
            model: "gpt-4".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
            messages: vec![Message {
                role: "user",
                content: "Hello, world!".to_string(),
            }],
        };

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["model"], "gpt-4");
        assert_eq!(json["max_tokens"], 4096);
        assert_eq!(json["temperature"], 0.7);
        assert_eq!(json["messages"].as_array().unwrap().len(), 1);
        assert_eq!(json["messages"][0]["role"], "user");
        assert_eq!(json["messages"][0]["content"], "Hello, world!");
    }

    #[test]
    fn test_request_serialization_with_special_characters() {
        let request = Request {
            model: "gpt-3.5-turbo".to_string(),
            max_tokens: 1024,
            temperature: 0.5,
            messages: vec![Message {
                role: "user",
                content:
                    r#"Parse this JSON: {"key": "value"} and handle "quotes" and \backslashes\"#
                        .to_string(),
            }],
        };

        let json_str = serde_json::to_string(&request).unwrap();
        // Verify it can be round-tripped
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert!(parsed["messages"][0]["content"]
            .as_str()
            .unwrap()
            .contains(r#"{"key": "value"}"#));
    }

    #[test]
    fn test_response_deserialization_single_choice() {
        let json = r#"{
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1677652288,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello! How can I help you?"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 15,
                "total_tokens": 25
            }
        }"#;

        let response: Response = serde_json::from_str(json).unwrap();
        assert_eq!(response.choices.len(), 1);
        assert_eq!(
            response.choices[0].message.content,
            "Hello! How can I help you?"
        );
    }

    #[test]
    fn test_response_deserialization_multiple_choices() {
        let json = r#"{
            "id": "chatcmpl-456",
            "object": "chat.completion",
            "created": 1677652288,
            "model": "gpt-4",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "First response"
                    },
                    "finish_reason": "stop"
                },
                {
                    "index": 1,
                    "message": {
                        "role": "assistant",
                        "content": "Second response"
                    },
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 20,
                "total_tokens": 30
            }
        }"#;

        let response: Response = serde_json::from_str(json).unwrap();
        assert_eq!(response.choices.len(), 2);
        assert_eq!(response.choices[0].message.content, "First response");
        assert_eq!(response.choices[1].message.content, "Second response");
    }

    #[test]
    fn test_response_deserialization_empty_choices() {
        let json = r#"{
            "id": "chatcmpl-789",
            "object": "chat.completion",
            "created": 1677652288,
            "model": "gpt-4",
            "choices": [],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 0,
                "total_tokens": 10
            }
        }"#;

        let response: Response = serde_json::from_str(json).unwrap();
        assert!(response.choices.is_empty());
    }

    #[test]
    fn test_response_deserialization_with_unicode() {
        let json = r#"{
            "id": "chatcmpl-unicode",
            "object": "chat.completion",
            "created": 1677652288,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Here's some Unicode: 你好世界 🌍 émojis"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 15,
                "total_tokens": 25
            }
        }"#;

        let response: Response = serde_json::from_str(json).unwrap();
        assert!(response.choices[0].message.content.contains("你好世界"));
        assert!(response.choices[0].message.content.contains("🌍"));
    }

    #[test]
    fn test_empty_choices_returns_empty_string() {
        // Simulate what happens when choices is empty
        let response = Response { choices: vec![] };
        let result: String = response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_default();
        assert_eq!(result, "");
    }

    // ==================== Client Construction Tests ====================

    #[test]
    fn test_client_construction() {
        let client = OpenAi::new("test-api-key", "gpt-4", 4096, 0.7);

        assert_eq!(client.api_key, "test-api-key");
        assert_eq!(client.model, "gpt-4");
        assert_eq!(client.max_tokens, 4096);
        assert_eq!(client.temperature, 0.7);
    }

    #[test]
    fn test_client_name() {
        let client = OpenAi::new("key", "model", 100, 0.1);
        assert_eq!(client.name(), "openai");
    }

    #[test]
    fn test_from_env_missing_key() {
        // Ensure the env var is not set for this test
        std::env::remove_var("OPENAI_API_KEY");

        let result = OpenAi::from_env("gpt-4");
        assert!(result.is_err());

        let err = result.unwrap_err();
        let err_str = err.to_string();
        assert!(
            err_str.contains("OPENAI_API_KEY"),
            "Error should mention missing env var: {}",
            err_str
        );
    }

    // ==================== Error Response Parsing Tests ====================

    #[test]
    fn test_malformed_response_missing_choices() {
        let json = r#"{
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "model": "gpt-4"
        }"#;

        let result: std::result::Result<Response, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_malformed_response_wrong_choices_type() {
        let json = r#"{
            "choices": "this should be an array"
        }"#;

        let result: std::result::Result<Response, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    // ==================== API Constants Tests ====================

    #[test]
    fn test_api_url_is_valid() {
        assert!(API_URL.starts_with("https://"));
        assert!(API_URL.contains("openai.com"));
        assert!(API_URL.contains("/v1/chat/completions"));
    }

    // ==================== Temperature Validation Tests ====================

    #[test]
    fn test_temperature_boundaries() {
        // OpenAI accepts temperature from 0.0 to 2.0
        let client_low = OpenAi::new("key", "gpt-4", 100, 0.0);
        assert_eq!(client_low.temperature, 0.0);

        let client_high = OpenAi::new("key", "gpt-4", 100, 2.0);
        assert_eq!(client_high.temperature, 2.0);

        let client_mid = OpenAi::new("key", "gpt-4", 100, 1.0);
        assert_eq!(client_mid.temperature, 1.0);
    }
}

/// Integration tests that require real API access.
/// Run with: `cargo test --features integration-tests -- --ignored`
#[cfg(all(test, feature = "integration-tests"))]
mod integration_tests {
    use super::*;
    use std::time::Duration;

    /// Helper to create a client from environment.
    /// Requires OPENAI_API_KEY to be set.
    fn create_test_client() -> Option<OpenAi> {
        match OpenAi::from_env("gpt-4o-mini") {
            Ok(client) => Some(client),
            Err(e) => {
                eprintln!("Skipping OpenAI integration test: {}", e);
                None
            }
        }
    }

    #[tokio::test]
    #[ignore = "requires OPENAI_API_KEY and network access"]
    async fn test_simple_completion() {
        let Some(client) = create_test_client() else {
            return;
        };

        let result = tokio::time::timeout(
            Duration::from_secs(30),
            client.complete("Say 'hello' and nothing else."),
        )
        .await
        .expect("Request timed out")
        .expect("API call failed");

        assert!(
            result.to_lowercase().contains("hello"),
            "Expected 'hello' in response: {}",
            result
        );
    }

    #[tokio::test]
    #[ignore = "requires OPENAI_API_KEY and network access"]
    async fn test_json_response_parsing() {
        let Some(client) = create_test_client() else {
            return;
        };

        let prompt =
            r#"Respond with exactly this JSON and nothing else: {"status": "ok", "count": 42}"#;

        let result = tokio::time::timeout(Duration::from_secs(30), client.complete(prompt))
            .await
            .expect("Request timed out")
            .expect("API call failed");

        // Try to parse as JSON
        let parsed: std::result::Result<serde_json::Value, _> = serde_json::from_str(&result);
        assert!(parsed.is_ok(), "Expected valid JSON response: {}", result);

        let json = parsed.unwrap();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["count"], 42);
    }

    #[tokio::test]
    #[ignore = "requires OPENAI_API_KEY and network access"]
    async fn test_long_prompt_handling() {
        let Some(client) = create_test_client() else {
            return;
        };

        // Create a moderately long prompt (not too long to avoid rate limits)
        let long_text = "word ".repeat(500);
        let prompt = format!(
            "Count the approximate number of words in the following text and respond with just a number: {}",
            long_text
        );

        let result = tokio::time::timeout(Duration::from_secs(30), client.complete(&prompt))
            .await
            .expect("Request timed out")
            .expect("API call failed");

        // Should contain some numeric response
        let has_number = result.chars().any(|c| c.is_ascii_digit());
        assert!(has_number, "Expected numeric response: {}", result);
    }

    #[tokio::test]
    #[ignore = "requires invalid API key test"]
    async fn test_invalid_api_key_error() {
        let client = OpenAi::new("invalid-key-12345", "gpt-4o-mini", 100, 0.1);

        let result = client.complete("test").await;
        assert!(result.is_err(), "Expected error with invalid API key");

        let err = result.err().unwrap();
        // Should be a connection/HTTP error
        assert!(
            matches!(err, Error::Connection(_) | Error::Http(_)),
            "Expected connection or HTTP error, got: {:?}",
            err
        );
    }
}
