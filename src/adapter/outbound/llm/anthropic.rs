//! Anthropic Claude LLM client.
//!
//! Provides an implementation of the [`Llm`] trait for the Anthropic
//! Claude API, supporting text completion requests.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::port::outbound::llm::Llm;

/// Anthropic Messages API endpoint.
const API_URL: &str = "https://api.anthropic.com/v1/messages";

/// API version header value.
const API_VERSION: &str = "2023-06-01";

/// Anthropic Claude API client.
///
/// Implements the [`Llm`] trait for making completion requests to the
/// Anthropic Messages API.
#[derive(Debug)]
pub struct Anthropic {
    /// HTTP client for API requests.
    client: Client,
    /// API key for authentication.
    api_key: String,
    /// Model identifier (e.g., "claude-sonnet-4-6").
    model: String,
    /// Maximum tokens to generate in the response.
    max_tokens: usize,
    /// Sampling temperature (0.0 to 1.0).
    temperature: f64,
}

impl Anthropic {
    /// Create a new Anthropic client with explicit configuration.
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

    /// Create a client from the `ANTHROPIC_API_KEY` environment variable.
    ///
    /// # Errors
    ///
    /// Returns an error if the environment variable is not set.
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
impl Llm for Anthropic {
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

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Request/Response Serialization Tests ====================

    #[test]
    fn test_request_serialization() {
        let request = Request {
            model: "claude-sonnet-4-6".to_string(),
            max_tokens: 4096,
            temperature: 0.2,
            messages: vec![Message {
                role: "user",
                content: "Hello, world!".to_string(),
            }],
        };

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["model"], "claude-sonnet-4-6");
        assert_eq!(json["max_tokens"], 4096);
        assert_eq!(json["temperature"], 0.2);
        assert_eq!(json["messages"].as_array().unwrap().len(), 1);
        assert_eq!(json["messages"][0]["role"], "user");
        assert_eq!(json["messages"][0]["content"], "Hello, world!");
    }

    #[test]
    fn test_request_serialization_with_special_characters() {
        let request = Request {
            model: "claude-sonnet-4-6".to_string(),
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
    fn test_response_deserialization_single_content_block() {
        let json = r#"{
            "content": [
                {"type": "text", "text": "Hello, I'm Claude!"}
            ],
            "id": "msg_123",
            "model": "claude-sonnet-4-6",
            "role": "assistant",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "type": "message",
            "usage": {"input_tokens": 10, "output_tokens": 20}
        }"#;

        let response: Response = serde_json::from_str(json).unwrap();
        assert_eq!(response.content.len(), 1);
        assert_eq!(response.content[0].text, "Hello, I'm Claude!");
    }

    #[test]
    fn test_response_deserialization_multiple_content_blocks() {
        let json = r#"{
            "content": [
                {"type": "text", "text": "First part. "},
                {"type": "text", "text": "Second part."}
            ],
            "id": "msg_456",
            "model": "claude-sonnet-4-6",
            "role": "assistant",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "type": "message",
            "usage": {"input_tokens": 10, "output_tokens": 30}
        }"#;

        let response: Response = serde_json::from_str(json).unwrap();
        assert_eq!(response.content.len(), 2);

        let combined: String = response.content.iter().map(|c| c.text.as_str()).collect();
        assert_eq!(combined, "First part. Second part.");
    }

    #[test]
    fn test_response_deserialization_empty_content() {
        let json = r#"{
            "content": [],
            "id": "msg_789",
            "model": "claude-sonnet-4-6",
            "role": "assistant",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "type": "message",
            "usage": {"input_tokens": 10, "output_tokens": 0}
        }"#;

        let response: Response = serde_json::from_str(json).unwrap();
        assert!(response.content.is_empty());
    }

    #[test]
    fn test_response_deserialization_with_unicode() {
        let json = r#"{
            "content": [
                {"type": "text", "text": "Here's some Unicode: 你好世界 🌍 émojis"}
            ],
            "id": "msg_unicode",
            "model": "claude-sonnet-4-6",
            "role": "assistant",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "type": "message",
            "usage": {"input_tokens": 10, "output_tokens": 15}
        }"#;

        let response: Response = serde_json::from_str(json).unwrap();
        assert!(response.content[0].text.contains("你好世界"));
        assert!(response.content[0].text.contains("🌍"));
    }

    // ==================== Client Construction Tests ====================

    #[test]
    fn test_client_construction() {
        let client = Anthropic::new("test-api-key", "claude-sonnet-4-6", 4096, 0.5);

        assert_eq!(client.api_key, "test-api-key");
        assert_eq!(client.model, "claude-sonnet-4-6");
        assert_eq!(client.max_tokens, 4096);
        assert_eq!(client.temperature, 0.5);
    }

    #[test]
    fn test_client_name() {
        let client = Anthropic::new("key", "model", 100, 0.1);
        assert_eq!(client.name(), "anthropic");
    }

    #[test]
    fn test_from_env_missing_key() {
        // Ensure the env var is not set for this test
        std::env::remove_var("ANTHROPIC_API_KEY");

        let result = Anthropic::from_env("claude-sonnet-4-6");
        assert!(result.is_err());

        let err = result.unwrap_err();
        let err_str = err.to_string();
        assert!(
            err_str.contains("ANTHROPIC_API_KEY"),
            "Error should mention missing env var: {}",
            err_str
        );
    }

    // ==================== Error Response Parsing Tests ====================

    #[test]
    fn test_malformed_response_missing_content() {
        let json = r#"{
            "id": "msg_123",
            "model": "claude-sonnet-4-6"
        }"#;

        let result: std::result::Result<Response, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_malformed_response_wrong_content_type() {
        let json = r#"{
            "content": "this should be an array"
        }"#;

        let result: std::result::Result<Response, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    // ==================== API Constants Tests ====================

    #[test]
    fn test_api_url_is_valid() {
        assert!(API_URL.starts_with("https://"));
        assert!(API_URL.contains("anthropic.com"));
        assert!(API_URL.contains("/v1/messages"));
    }

    #[test]
    fn test_api_version_format() {
        // API version should be a date in YYYY-MM-DD format
        assert!(API_VERSION.len() == 10);
        assert!(API_VERSION.chars().filter(|c| *c == '-').count() == 2);
    }
}

/// Integration tests that require real API access.
/// Run with: `cargo test --features integration-tests -- --ignored`
#[cfg(all(test, feature = "integration-tests"))]
mod integration_tests {
    use super::*;
    use std::time::Duration;

    /// Helper to create a client from environment.
    /// Requires ANTHROPIC_API_KEY to be set.
    fn create_test_client() -> Option<Anthropic> {
        match Anthropic::from_env("claude-haiku-4-5") {
            Ok(client) => Some(client),
            Err(e) => {
                eprintln!("Skipping Anthropic integration test: {}", e);
                None
            }
        }
    }

    #[tokio::test]
    #[ignore = "requires ANTHROPIC_API_KEY and network access"]
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
    #[ignore = "requires ANTHROPIC_API_KEY and network access"]
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
    #[ignore = "requires ANTHROPIC_API_KEY and network access"]
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
        let client = Anthropic::new("invalid-key-12345", "claude-haiku-4-5", 100, 0.1);

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
