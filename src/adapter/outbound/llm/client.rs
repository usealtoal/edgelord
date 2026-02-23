//! LLM client utilities and test helpers.
//!
//! Provides shared utilities for LLM clients and mock implementations
//! for testing.

/// Mock LLM for testing.
#[cfg(test)]
pub mod tests {
    use async_trait::async_trait;

    use crate::error::Result;
    use crate::port::outbound::llm::Llm;

    /// Mock LLM client that returns a predefined response.
    ///
    /// Useful for testing code that depends on LLM completions without
    /// making actual API calls.
    pub struct MockLlm {
        response: String,
    }

    impl MockLlm {
        /// Create a new mock LLM with the given response.
        pub fn new(response: impl Into<String>) -> Self {
            Self {
                response: response.into(),
            }
        }
    }

    #[async_trait]
    impl Llm for MockLlm {
        fn name(&self) -> &'static str {
            "mock"
        }

        async fn complete(&self, _prompt: &str) -> Result<String> {
            Ok(self.response.clone())
        }
    }
}

#[cfg(test)]
mod internal_tests {
    use super::tests::MockLlm;
    use crate::port::outbound::llm::Llm;

    #[tokio::test]
    async fn mock_llm_returns_response() {
        let llm = MockLlm::new(r#"{"constraints": []}"#);
        let result = llm.complete("test").await.unwrap();
        assert_eq!(result, r#"{"constraints": []}"#);
    }
}
