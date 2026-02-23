//! Integration tests for LLM adapters.
//!
//! These tests require real API keys and network access to run.
//!
//! # Running Integration Tests
//!
//! Integration tests are gated behind the `integration-tests` feature flag
//! and are marked with `#[ignore]` to prevent accidental execution.
//!
//! ## Prerequisites
//!
//! Set the required environment variables:
//! ```bash
//! export ANTHROPIC_API_KEY="your-anthropic-api-key"
//! export OPENAI_API_KEY="your-openai-api-key"
//! ```
//!
//! ## Running All Integration Tests
//!
//! ```bash
//! cargo test --features integration-tests -- --ignored
//! ```
//!
//! ## Running Specific Provider Tests
//!
//! ```bash
//! # Anthropic only
//! cargo test --features integration-tests anthropic -- --ignored
//!
//! # OpenAI only
//! cargo test --features integration-tests openai -- --ignored
//! ```
//!
//! ## Running with Verbose Output
//!
//! ```bash
//! cargo test --features integration-tests -- --ignored --nocapture
//! ```
//!
//! # Cost Considerations
//!
//! These tests make real API calls that incur costs. The tests are designed
//! to use minimal tokens (cheap models, short prompts) but will still result
//! in small charges on your API accounts.
//!
//! # Test Isolation
//!
//! Each test is independent and can be run in isolation. Tests do not share
//! state and do not depend on each other's execution order.

#![cfg(feature = "integration-tests")]

use std::time::Duration;

use edgelord::adapter::outbound::llm::anthropic::Anthropic;
use edgelord::adapter::outbound::llm::openai::OpenAi;
use edgelord::port::outbound::llm::Llm;

// ============================================================================
// Anthropic Integration Tests
// ============================================================================

mod anthropic_integration {
    use super::*;

    /// Create a test client using environment variables.
    /// Uses claude-3-haiku for cost efficiency.
    fn create_client() -> Option<Anthropic> {
        match Anthropic::from_env("claude-3-haiku-20240307") {
            Ok(client) => Some(client),
            Err(e) => {
                eprintln!("Skipping Anthropic test: {}", e);
                None
            }
        }
    }

    #[tokio::test]
    #[ignore = "requires ANTHROPIC_API_KEY and network access"]
    async fn test_basic_completion() {
        let Some(client) = create_client() else {
            return;
        };

        let result = tokio::time::timeout(
            Duration::from_secs(30),
            client.complete("Respond with exactly: PONG"),
        )
        .await
        .expect("Request timed out")
        .expect("API call failed");

        assert!(
            result.contains("PONG"),
            "Expected 'PONG' in response: {}",
            result
        );
    }

    #[tokio::test]
    #[ignore = "requires ANTHROPIC_API_KEY and network access"]
    async fn test_json_output() {
        let Some(client) = create_client() else {
            return;
        };

        let prompt = r#"Output exactly this JSON with no other text:
{"status":"success","value":123}"#;

        let result = tokio::time::timeout(Duration::from_secs(30), client.complete(prompt))
            .await
            .expect("Request timed out")
            .expect("API call failed");

        // Parse the JSON
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result);
        assert!(parsed.is_ok(), "Expected valid JSON, got: {}", result);

        let json = parsed.unwrap();
        assert_eq!(json["status"], "success");
        assert_eq!(json["value"], 123);
    }

    #[tokio::test]
    #[ignore = "requires ANTHROPIC_API_KEY and network access"]
    async fn test_multiline_response() {
        let Some(client) = create_client() else {
            return;
        };

        let result = tokio::time::timeout(
            Duration::from_secs(30),
            client.complete("List exactly 3 colors, one per line, nothing else:"),
        )
        .await
        .expect("Request timed out")
        .expect("API call failed");

        let lines: Vec<&str> = result.lines().filter(|l| !l.trim().is_empty()).collect();
        assert!(
            lines.len() >= 3,
            "Expected at least 3 lines, got {}: {}",
            lines.len(),
            result
        );
    }

    #[tokio::test]
    #[ignore = "requires ANTHROPIC_API_KEY and network access"]
    async fn test_unicode_handling() {
        let Some(client) = create_client() else {
            return;
        };

        let result = tokio::time::timeout(
            Duration::from_secs(30),
            client.complete("What is 'hello' in Japanese? Reply with just the word."),
        )
        .await
        .expect("Request timed out")
        .expect("API call failed");

        // Should contain Japanese characters
        let has_japanese = result.chars().any(|c| {
            matches!(c,
                '\u{3040}'..='\u{309F}' |  // Hiragana
                '\u{30A0}'..='\u{30FF}' |  // Katakana
                '\u{4E00}'..='\u{9FFF}'    // CJK
            )
        });
        assert!(
            has_japanese,
            "Expected Japanese characters in response: {}",
            result
        );
    }

    #[tokio::test]
    #[ignore = "requires ANTHROPIC_API_KEY and network access"]
    async fn test_math_reasoning() {
        let Some(client) = create_client() else {
            return;
        };

        let result = tokio::time::timeout(
            Duration::from_secs(30),
            client.complete("What is 15 + 27? Reply with just the number."),
        )
        .await
        .expect("Request timed out")
        .expect("API call failed");

        assert!(
            result.contains("42"),
            "Expected '42' in response: {}",
            result
        );
    }

    #[tokio::test]
    #[ignore = "requires ANTHROPIC_API_KEY and network access"]
    async fn test_client_name_is_correct() {
        let Some(client) = create_client() else {
            return;
        };

        assert_eq!(client.name(), "anthropic");
    }
}

// ============================================================================
// OpenAI Integration Tests
// ============================================================================

mod openai_integration {
    use super::*;

    /// Create a test client using environment variables.
    /// Uses gpt-4o-mini for cost efficiency.
    fn create_client() -> Option<OpenAi> {
        match OpenAi::from_env("gpt-4o-mini") {
            Ok(client) => Some(client),
            Err(e) => {
                eprintln!("Skipping OpenAI test: {}", e);
                None
            }
        }
    }

    #[tokio::test]
    #[ignore = "requires OPENAI_API_KEY and network access"]
    async fn test_basic_completion() {
        let Some(client) = create_client() else {
            return;
        };

        let result = tokio::time::timeout(
            Duration::from_secs(30),
            client.complete("Respond with exactly: PONG"),
        )
        .await
        .expect("Request timed out")
        .expect("API call failed");

        assert!(
            result.contains("PONG"),
            "Expected 'PONG' in response: {}",
            result
        );
    }

    #[tokio::test]
    #[ignore = "requires OPENAI_API_KEY and network access"]
    async fn test_json_output() {
        let Some(client) = create_client() else {
            return;
        };

        let prompt = r#"Output exactly this JSON with no other text:
{"status":"success","value":123}"#;

        let result = tokio::time::timeout(Duration::from_secs(30), client.complete(prompt))
            .await
            .expect("Request timed out")
            .expect("API call failed");

        // Parse the JSON
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result);
        assert!(parsed.is_ok(), "Expected valid JSON, got: {}", result);

        let json = parsed.unwrap();
        assert_eq!(json["status"], "success");
        assert_eq!(json["value"], 123);
    }

    #[tokio::test]
    #[ignore = "requires OPENAI_API_KEY and network access"]
    async fn test_multiline_response() {
        let Some(client) = create_client() else {
            return;
        };

        let result = tokio::time::timeout(
            Duration::from_secs(30),
            client.complete("List exactly 3 colors, one per line, nothing else:"),
        )
        .await
        .expect("Request timed out")
        .expect("API call failed");

        let lines: Vec<&str> = result.lines().filter(|l| !l.trim().is_empty()).collect();
        assert!(
            lines.len() >= 3,
            "Expected at least 3 lines, got {}: {}",
            lines.len(),
            result
        );
    }

    #[tokio::test]
    #[ignore = "requires OPENAI_API_KEY and network access"]
    async fn test_unicode_handling() {
        let Some(client) = create_client() else {
            return;
        };

        let result = tokio::time::timeout(
            Duration::from_secs(30),
            client.complete("What is 'hello' in Japanese? Reply with just the word."),
        )
        .await
        .expect("Request timed out")
        .expect("API call failed");

        // Should contain Japanese characters
        let has_japanese = result.chars().any(|c| {
            matches!(c,
                '\u{3040}'..='\u{309F}' |  // Hiragana
                '\u{30A0}'..='\u{30FF}' |  // Katakana
                '\u{4E00}'..='\u{9FFF}'    // CJK
            )
        });
        assert!(
            has_japanese,
            "Expected Japanese characters in response: {}",
            result
        );
    }

    #[tokio::test]
    #[ignore = "requires OPENAI_API_KEY and network access"]
    async fn test_math_reasoning() {
        let Some(client) = create_client() else {
            return;
        };

        let result = tokio::time::timeout(
            Duration::from_secs(30),
            client.complete("What is 15 + 27? Reply with just the number."),
        )
        .await
        .expect("Request timed out")
        .expect("API call failed");

        assert!(
            result.contains("42"),
            "Expected '42' in response: {}",
            result
        );
    }

    #[tokio::test]
    #[ignore = "requires OPENAI_API_KEY and network access"]
    async fn test_client_name_is_correct() {
        let Some(client) = create_client() else {
            return;
        };

        assert_eq!(client.name(), "openai");
    }
}

// ============================================================================
// Cross-Provider Tests
// ============================================================================

mod cross_provider {
    use super::*;

    /// Test that both providers can handle the same prompt and return valid responses.
    #[tokio::test]
    #[ignore = "requires both API keys and network access"]
    async fn test_same_prompt_both_providers() {
        let anthropic = Anthropic::from_env("claude-3-haiku-20240307").ok();
        let openai = OpenAi::from_env("gpt-4o-mini").ok();

        let prompt = "What is 2 + 2? Reply with just the number.";

        if let Some(client) = anthropic {
            let result = tokio::time::timeout(Duration::from_secs(30), client.complete(prompt))
                .await
                .expect("Anthropic timed out")
                .expect("Anthropic call failed");
            assert!(
                result.contains("4"),
                "Anthropic should return 4: {}",
                result
            );
        }

        if let Some(client) = openai {
            let result = tokio::time::timeout(Duration::from_secs(30), client.complete(prompt))
                .await
                .expect("OpenAI timed out")
                .expect("OpenAI call failed");
            assert!(result.contains("4"), "OpenAI should return 4: {}", result);
        }
    }

    /// Test that the Llm trait can be used polymorphically.
    #[tokio::test]
    #[ignore = "requires at least one API key and network access"]
    async fn test_trait_polymorphism() {
        let clients: Vec<Box<dyn Llm>> = vec![
            Anthropic::from_env("claude-3-haiku-20240307")
                .ok()
                .map(|c| Box::new(c) as Box<dyn Llm>),
            OpenAi::from_env("gpt-4o-mini")
                .ok()
                .map(|c| Box::new(c) as Box<dyn Llm>),
        ]
        .into_iter()
        .flatten()
        .collect();

        if clients.is_empty() {
            eprintln!("No API keys available, skipping polymorphism test");
            return;
        }

        for client in clients {
            let result = tokio::time::timeout(Duration::from_secs(30), client.complete("Say 'hi'"))
                .await
                .expect("Request timed out")
                .expect("API call failed");

            assert!(
                !result.is_empty(),
                "{} returned empty response",
                client.name()
            );
        }
    }
}

// ============================================================================
// Error Handling Integration Tests
// ============================================================================

mod error_handling {
    use super::*;

    #[tokio::test]
    #[ignore = "tests error handling with invalid credentials"]
    async fn test_anthropic_invalid_key() {
        let client = Anthropic::new("sk-invalid-key-12345", "claude-3-haiku-20240307", 100, 0.1);

        let result = client.complete("test").await;
        assert!(result.is_err(), "Expected error with invalid API key");

        let err = result.unwrap_err();
        eprintln!("Anthropic invalid key error: {}", err);
    }

    #[tokio::test]
    #[ignore = "tests error handling with invalid credentials"]
    async fn test_openai_invalid_key() {
        let client = OpenAi::new("sk-invalid-key-12345", "gpt-4o-mini", 100, 0.1);

        let result = client.complete("test").await;
        assert!(result.is_err(), "Expected error with invalid API key");

        let err = result.unwrap_err();
        eprintln!("OpenAI invalid key error: {}", err);
    }

    #[tokio::test]
    #[ignore = "tests error handling with invalid model"]
    async fn test_anthropic_invalid_model() {
        let Ok(client) = Anthropic::from_env("nonexistent-model-xyz") else {
            eprintln!("Skipping: ANTHROPIC_API_KEY not set");
            return;
        };

        let result = client.complete("test").await;
        assert!(result.is_err(), "Expected error with invalid model");
    }

    #[tokio::test]
    #[ignore = "tests error handling with invalid model"]
    async fn test_openai_invalid_model() {
        let Ok(client) = OpenAi::from_env("nonexistent-model-xyz") else {
            eprintln!("Skipping: OPENAI_API_KEY not set");
            return;
        };

        let result = client.complete("test").await;
        assert!(result.is_err(), "Expected error with invalid model");
    }
}

// ============================================================================
// Performance Tests (Optional)
// ============================================================================

mod performance {
    use super::*;

    /// Test response time is within acceptable bounds.
    /// Note: This is a soft test - network conditions vary.
    #[tokio::test]
    #[ignore = "requires API keys and measures latency"]
    async fn test_response_latency() {
        let anthropic = Anthropic::from_env("claude-3-haiku-20240307").ok();
        let openai = OpenAi::from_env("gpt-4o-mini").ok();

        let prompt = "Reply with OK";

        if let Some(client) = anthropic {
            let start = std::time::Instant::now();
            let _ = client.complete(prompt).await;
            let duration = start.elapsed();
            eprintln!("Anthropic latency: {:?}", duration);
            // Soft assertion - just warn if slow
            if duration > Duration::from_secs(10) {
                eprintln!("WARNING: Anthropic response was slow (>10s)");
            }
        }

        if let Some(client) = openai {
            let start = std::time::Instant::now();
            let _ = client.complete(prompt).await;
            let duration = start.elapsed();
            eprintln!("OpenAI latency: {:?}", duration);
            if duration > Duration::from_secs(10) {
                eprintln!("WARNING: OpenAI response was slow (>10s)");
            }
        }
    }
}
