//! LLM client utilities and test helpers.
//!
//! Provides shared utilities for LLM clients and mock implementations
//! for testing.

#[cfg(any(test, feature = "testkit"))]
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(any(test, feature = "testkit"))]
use std::sync::Arc;

#[cfg(any(test, feature = "testkit"))]
use async_trait::async_trait;

#[cfg(any(test, feature = "testkit"))]
use crate::error::{Error, Result};
#[cfg(any(test, feature = "testkit"))]
use crate::port::outbound::llm::Llm;

/// Mock LLM client that returns a predefined response.
///
/// Useful for testing code that depends on LLM completions without
/// making actual API calls.
#[cfg(any(test, feature = "testkit"))]
pub struct MockLlm {
    response: String,
}

#[cfg(any(test, feature = "testkit"))]
impl MockLlm {
    /// Create a new mock LLM with the given response.
    pub fn new(response: impl Into<String>) -> Self {
        Self {
            response: response.into(),
        }
    }
}

#[cfg(any(test, feature = "testkit"))]
#[async_trait]
impl Llm for MockLlm {
    fn name(&self) -> &'static str {
        "mock"
    }

    async fn complete(&self, _prompt: &str) -> Result<String> {
        Ok(self.response.clone())
    }
}

/// Mock LLM that fails a configurable number of times before succeeding.
///
/// Useful for testing retry logic and error handling.
#[cfg(any(test, feature = "testkit"))]
pub struct FailingMockLlm {
    /// Number of times to fail before succeeding.
    failures_remaining: AtomicUsize,
    /// Response to return after failures are exhausted.
    success_response: String,
    /// Error message for failures.
    error_message: String,
}

#[cfg(any(test, feature = "testkit"))]
impl FailingMockLlm {
    /// Create a mock that fails `n` times before returning the success response.
    pub fn new(fail_count: usize, success_response: impl Into<String>) -> Self {
        Self {
            failures_remaining: AtomicUsize::new(fail_count),
            success_response: success_response.into(),
            error_message: "Simulated API failure".to_string(),
        }
    }

    /// Create a mock with a custom error message.
    pub fn with_error_message(
        fail_count: usize,
        success_response: impl Into<String>,
        error_message: impl Into<String>,
    ) -> Self {
        Self {
            failures_remaining: AtomicUsize::new(fail_count),
            success_response: success_response.into(),
            error_message: error_message.into(),
        }
    }

    /// Get the number of remaining failures.
    pub fn failures_remaining(&self) -> usize {
        self.failures_remaining.load(Ordering::SeqCst)
    }
}

#[cfg(any(test, feature = "testkit"))]
#[async_trait]
impl Llm for FailingMockLlm {
    fn name(&self) -> &'static str {
        "failing_mock"
    }

    async fn complete(&self, _prompt: &str) -> Result<String> {
        let remaining = self.failures_remaining.load(Ordering::SeqCst);
        if remaining > 0 {
            self.failures_remaining.fetch_sub(1, Ordering::SeqCst);
            Err(Error::Connection(self.error_message.clone()))
        } else {
            Ok(self.success_response.clone())
        }
    }
}

/// Mock LLM that tracks the number of calls made.
///
/// Useful for testing rate limiting and call counting.
#[cfg(any(test, feature = "testkit"))]
pub struct CountingMockLlm {
    /// Number of calls made.
    call_count: AtomicUsize,
    /// Response to return.
    response: String,
}

#[cfg(any(test, feature = "testkit"))]
impl CountingMockLlm {
    /// Create a new counting mock LLM.
    pub fn new(response: impl Into<String>) -> Self {
        Self {
            call_count: AtomicUsize::new(0),
            response: response.into(),
        }
    }

    /// Get the number of calls made to this mock.
    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }

    /// Reset the call counter.
    pub fn reset(&self) {
        self.call_count.store(0, Ordering::SeqCst);
    }
}

#[cfg(any(test, feature = "testkit"))]
#[async_trait]
impl Llm for CountingMockLlm {
    fn name(&self) -> &'static str {
        "counting_mock"
    }

    async fn complete(&self, _prompt: &str) -> Result<String> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        Ok(self.response.clone())
    }
}

/// Mock LLM that records prompts for later inspection.
///
/// Useful for verifying the prompts sent to the LLM.
#[cfg(any(test, feature = "testkit"))]
pub struct RecordingMockLlm {
    /// Recorded prompts.
    prompts: parking_lot::Mutex<Vec<String>>,
    /// Response to return.
    response: String,
}

#[cfg(any(test, feature = "testkit"))]
impl RecordingMockLlm {
    /// Create a new recording mock LLM.
    pub fn new(response: impl Into<String>) -> Self {
        Self {
            prompts: parking_lot::Mutex::new(Vec::new()),
            response: response.into(),
        }
    }

    /// Get all recorded prompts.
    pub fn prompts(&self) -> Vec<String> {
        self.prompts.lock().clone()
    }

    /// Get the last prompt sent, if any.
    pub fn last_prompt(&self) -> Option<String> {
        self.prompts.lock().last().cloned()
    }

    /// Clear recorded prompts.
    pub fn clear(&self) {
        self.prompts.lock().clear();
    }
}

#[cfg(any(test, feature = "testkit"))]
#[async_trait]
impl Llm for RecordingMockLlm {
    fn name(&self) -> &'static str {
        "recording_mock"
    }

    async fn complete(&self, prompt: &str) -> Result<String> {
        self.prompts.lock().push(prompt.to_string());
        Ok(self.response.clone())
    }
}

/// Mock LLM that simulates rate limiting.
///
/// Returns rate limit errors until reset.
#[cfg(any(test, feature = "testkit"))]
pub struct RateLimitedMockLlm {
    /// Whether rate limiting is active.
    rate_limited: std::sync::atomic::AtomicBool,
    /// Response to return when not rate limited.
    response: String,
}

#[cfg(any(test, feature = "testkit"))]
impl RateLimitedMockLlm {
    /// Create a new rate limited mock (starts rate limited).
    pub fn new(response: impl Into<String>) -> Self {
        Self {
            rate_limited: std::sync::atomic::AtomicBool::new(true),
            response: response.into(),
        }
    }

    /// Create a new mock that is not rate limited initially.
    pub fn not_limited(response: impl Into<String>) -> Self {
        Self {
            rate_limited: std::sync::atomic::AtomicBool::new(false),
            response: response.into(),
        }
    }

    /// Check if currently rate limited.
    pub fn is_rate_limited(&self) -> bool {
        self.rate_limited.load(Ordering::SeqCst)
    }

    /// Set rate limit status.
    pub fn set_rate_limited(&self, limited: bool) {
        self.rate_limited.store(limited, Ordering::SeqCst);
    }

    /// Clear the rate limit.
    pub fn reset(&self) {
        self.rate_limited.store(false, Ordering::SeqCst);
    }
}

#[cfg(any(test, feature = "testkit"))]
#[async_trait]
impl Llm for RateLimitedMockLlm {
    fn name(&self) -> &'static str {
        "rate_limited_mock"
    }

    async fn complete(&self, _prompt: &str) -> Result<String> {
        if self.rate_limited.load(Ordering::SeqCst) {
            Err(Error::Connection("Rate limit exceeded".to_string()))
        } else {
            Ok(self.response.clone())
        }
    }
}

/// LLM client wrapper that implements retry logic with exponential backoff.
///
/// This is a reference implementation showing how retry logic could be added.
/// In practice, you might use a crate like `backoff` or `retry`.
#[cfg(any(test, feature = "testkit"))]
pub struct RetryingLlm<L: Llm> {
    inner: Arc<L>,
    max_retries: usize,
    base_delay_ms: u64,
}

#[cfg(any(test, feature = "testkit"))]
impl<L: Llm> RetryingLlm<L> {
    /// Create a new retrying wrapper.
    pub fn new(inner: Arc<L>, max_retries: usize, base_delay_ms: u64) -> Self {
        Self {
            inner,
            max_retries,
            base_delay_ms,
        }
    }
}

#[cfg(any(test, feature = "testkit"))]
#[async_trait]
impl<L: Llm> Llm for RetryingLlm<L> {
    fn name(&self) -> &'static str {
        "retrying"
    }

    async fn complete(&self, prompt: &str) -> Result<String> {
        let mut last_error = None;
        for attempt in 0..=self.max_retries {
            match self.inner.complete(prompt).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.max_retries {
                        let delay = self.base_delay_ms * (2_u64.pow(attempt as u32));
                        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                    }
                }
            }
        }
        Err(last_error.unwrap_or_else(|| Error::Connection("Unknown error".to_string())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== MockLlm Tests ====================

    #[tokio::test]
    async fn mock_llm_returns_response() {
        let llm = MockLlm::new(r#"{"constraints": []}"#);
        let result = llm.complete("test").await.unwrap();
        assert_eq!(result, r#"{"constraints": []}"#);
    }

    #[tokio::test]
    async fn mock_llm_ignores_prompt() {
        let llm = MockLlm::new("fixed response");
        let result1 = llm.complete("prompt 1").await.unwrap();
        let result2 = llm.complete("completely different prompt").await.unwrap();
        assert_eq!(result1, result2);
    }

    #[test]
    fn mock_llm_name() {
        let llm = MockLlm::new("test");
        assert_eq!(llm.name(), "mock");
    }

    // ==================== FailingMockLlm Tests ====================

    #[tokio::test]
    async fn failing_mock_fails_then_succeeds() {
        let llm = FailingMockLlm::new(2, "success!");

        // First two calls should fail
        assert!(llm.complete("test").await.is_err());
        assert!(llm.complete("test").await.is_err());

        // Third call should succeed
        let result = llm.complete("test").await.unwrap();
        assert_eq!(result, "success!");

        // Subsequent calls should also succeed
        let result = llm.complete("test").await.unwrap();
        assert_eq!(result, "success!");
    }

    #[tokio::test]
    async fn failing_mock_zero_failures() {
        let llm = FailingMockLlm::new(0, "immediate success");

        let result = llm.complete("test").await.unwrap();
        assert_eq!(result, "immediate success");
    }

    #[tokio::test]
    async fn failing_mock_custom_error() {
        let llm = FailingMockLlm::with_error_message(1, "success", "Custom error message");

        let err = llm.complete("test").await.unwrap_err();
        assert!(err.to_string().contains("Custom error message"));
    }

    #[tokio::test]
    async fn failing_mock_tracks_remaining() {
        let llm = FailingMockLlm::new(3, "success");

        assert_eq!(llm.failures_remaining(), 3);
        let _ = llm.complete("test").await;
        assert_eq!(llm.failures_remaining(), 2);
        let _ = llm.complete("test").await;
        assert_eq!(llm.failures_remaining(), 1);
        let _ = llm.complete("test").await;
        assert_eq!(llm.failures_remaining(), 0);
    }

    #[test]
    fn failing_mock_name() {
        let llm = FailingMockLlm::new(1, "test");
        assert_eq!(llm.name(), "failing_mock");
    }

    // ==================== CountingMockLlm Tests ====================

    #[tokio::test]
    async fn counting_mock_tracks_calls() {
        let llm = CountingMockLlm::new("response");

        assert_eq!(llm.call_count(), 0);
        let _ = llm.complete("test").await;
        assert_eq!(llm.call_count(), 1);
        let _ = llm.complete("test").await;
        let _ = llm.complete("test").await;
        assert_eq!(llm.call_count(), 3);
    }

    #[tokio::test]
    async fn counting_mock_reset() {
        let llm = CountingMockLlm::new("response");

        let _ = llm.complete("test").await;
        let _ = llm.complete("test").await;
        assert_eq!(llm.call_count(), 2);

        llm.reset();
        assert_eq!(llm.call_count(), 0);
    }

    #[test]
    fn counting_mock_name() {
        let llm = CountingMockLlm::new("test");
        assert_eq!(llm.name(), "counting_mock");
    }

    // ==================== RecordingMockLlm Tests ====================

    #[tokio::test]
    async fn recording_mock_records_prompts() {
        let llm = RecordingMockLlm::new("response");

        let _ = llm.complete("first prompt").await;
        let _ = llm.complete("second prompt").await;

        let prompts = llm.prompts();
        assert_eq!(prompts.len(), 2);
        assert_eq!(prompts[0], "first prompt");
        assert_eq!(prompts[1], "second prompt");
    }

    #[tokio::test]
    async fn recording_mock_last_prompt() {
        let llm = RecordingMockLlm::new("response");

        assert!(llm.last_prompt().is_none());

        let _ = llm.complete("first").await;
        assert_eq!(llm.last_prompt(), Some("first".to_string()));

        let _ = llm.complete("second").await;
        assert_eq!(llm.last_prompt(), Some("second".to_string()));
    }

    #[tokio::test]
    async fn recording_mock_clear() {
        let llm = RecordingMockLlm::new("response");

        let _ = llm.complete("prompt").await;
        assert_eq!(llm.prompts().len(), 1);

        llm.clear();
        assert!(llm.prompts().is_empty());
    }

    #[test]
    fn recording_mock_name() {
        let llm = RecordingMockLlm::new("test");
        assert_eq!(llm.name(), "recording_mock");
    }

    // ==================== RateLimitedMockLlm Tests ====================

    #[tokio::test]
    async fn rate_limited_mock_blocks_when_limited() {
        let llm = RateLimitedMockLlm::new("response");

        assert!(llm.is_rate_limited());
        let result = llm.complete("test").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Rate limit"));
    }

    #[tokio::test]
    async fn rate_limited_mock_succeeds_when_not_limited() {
        let llm = RateLimitedMockLlm::not_limited("response");

        assert!(!llm.is_rate_limited());
        let result = llm.complete("test").await.unwrap();
        assert_eq!(result, "response");
    }

    #[tokio::test]
    async fn rate_limited_mock_reset() {
        let llm = RateLimitedMockLlm::new("response");

        // Initially rate limited
        assert!(llm.complete("test").await.is_err());

        // Reset and try again
        llm.reset();
        let result = llm.complete("test").await.unwrap();
        assert_eq!(result, "response");
    }

    #[tokio::test]
    async fn rate_limited_mock_set_status() {
        let llm = RateLimitedMockLlm::not_limited("response");

        // Initially not limited
        assert!(llm.complete("test").await.is_ok());

        // Set limited
        llm.set_rate_limited(true);
        assert!(llm.complete("test").await.is_err());

        // Unset
        llm.set_rate_limited(false);
        assert!(llm.complete("test").await.is_ok());
    }

    #[test]
    fn rate_limited_mock_name() {
        let llm = RateLimitedMockLlm::new("test");
        assert_eq!(llm.name(), "rate_limited_mock");
    }

    // ==================== RetryingLlm Tests ====================

    #[tokio::test]
    async fn retrying_llm_succeeds_immediately() {
        let inner = Arc::new(MockLlm::new("success"));
        let retrying = RetryingLlm::new(inner, 3, 1);

        let result = retrying.complete("test").await.unwrap();
        assert_eq!(result, "success");
    }

    #[tokio::test]
    async fn retrying_llm_retries_on_failure() {
        let inner = Arc::new(FailingMockLlm::new(2, "success after retries"));
        let retrying = RetryingLlm::new(inner, 3, 1);

        let result = retrying.complete("test").await.unwrap();
        assert_eq!(result, "success after retries");
    }

    #[tokio::test]
    async fn retrying_llm_exhausts_retries() {
        let inner = Arc::new(FailingMockLlm::new(10, "never reached"));
        let retrying = RetryingLlm::new(inner, 2, 1);

        let result = retrying.complete("test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn retrying_llm_respects_max_retries() {
        let inner = Arc::new(CountingMockLlm::new("success"));
        let inner_clone = Arc::clone(&inner);
        let retrying = RetryingLlm::new(inner_clone, 3, 1);

        let _ = retrying.complete("test").await;
        // Should only call once since it succeeds immediately
        assert_eq!(inner.call_count(), 1);
    }

    #[test]
    fn retrying_llm_name() {
        let inner = Arc::new(MockLlm::new("test"));
        let retrying = RetryingLlm::new(inner, 3, 1);
        assert_eq!(retrying.name(), "retrying");
    }

    // ==================== Thread Safety Tests ====================

    #[tokio::test]
    async fn mock_llm_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MockLlm>();
        assert_send_sync::<FailingMockLlm>();
        assert_send_sync::<CountingMockLlm>();
        assert_send_sync::<RecordingMockLlm>();
        assert_send_sync::<RateLimitedMockLlm>();
    }

    #[tokio::test]
    async fn counting_mock_concurrent_calls() {
        let llm = Arc::new(CountingMockLlm::new("response"));

        let mut handles = Vec::new();
        for _ in 0..10 {
            let llm_clone = Arc::clone(&llm);
            handles.push(tokio::spawn(async move {
                let _ = llm_clone.complete("test").await;
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(llm.call_count(), 10);
    }

    // ==================== Error Handling Tests ====================

    #[tokio::test]
    async fn error_contains_context() {
        let llm = FailingMockLlm::with_error_message(1, "success", "API timeout after 30s");

        let err = llm.complete("test").await.unwrap_err();
        let err_str = err.to_string();
        assert!(err_str.contains("timeout") || err_str.contains("30s"));
    }
}
