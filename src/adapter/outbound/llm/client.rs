//! LLM client abstraction for inference.

/// Mock LLM for testing.
#[cfg(test)]
pub mod tests {
    use async_trait::async_trait;

    use crate::error::Result;
    use crate::port::outbound::llm::Llm;

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
    use crate::port::outbound::llm::Llm;

    #[tokio::test]
    async fn mock_llm_returns_response() {
        let llm = MockLlm::new(r#"{"constraints": []}"#);
        let result = llm.complete("test").await.unwrap();
        assert_eq!(result, r#"{"constraints": []}"#);
    }
}
