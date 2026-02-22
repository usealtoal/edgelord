//! LLM client abstraction for inference.

mod anthropic;
mod openai;

pub use anthropic::Anthropic;
pub use openai::OpenAi;

use async_trait::async_trait;

use crate::error::Result;

/// LLM completion client trait.
#[async_trait]
pub trait Llm: Send + Sync {
    /// Provider name for logging.
    fn name(&self) -> &'static str;

    /// Send a completion request and return the response text.
    async fn complete(&self, prompt: &str) -> Result<String>;
}

/// Mock LLM for testing.
#[cfg(test)]
pub mod tests {
    use super::*;

    pub struct MockLlm {
        response: String,
    }

    impl MockLlm {
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
    use super::*;

    #[tokio::test]
    async fn mock_llm_returns_response() {
        let llm = MockLlm::new(r#"{"constraints": []}"#);
        let result = llm.complete("test").await.unwrap();
        assert_eq!(result, r#"{"constraints": []}"#);
    }
}
