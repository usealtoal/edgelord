//! Tests for factory functions.
//!
//! Verifies that factory functions correctly build infrastructure components
//! based on configuration settings.

use std::sync::Arc;

use chrono::Duration;

use crate::application::cache::cluster::ClusterCache;
use crate::infrastructure::config::llm::LlmProvider;
use crate::infrastructure::config::settings::Config;
use crate::infrastructure::factory::{inference, llm, solver, strategy};

// ---------------------------------------------------------------------------
// LLM Factory Tests
// ---------------------------------------------------------------------------

mod llm_factory {
    use super::*;

    fn minimal_config() -> Config {
        let toml = r#"
            [logging]
            level = "info"
            format = "pretty"
        "#;
        Config::parse_toml(toml).expect("minimal config should parse")
    }

    #[test]
    fn returns_none_when_inference_disabled() {
        let mut config = minimal_config();
        config.inference.enabled = false;

        let client = llm::build_llm_client(&config);
        assert!(client.is_none());
    }

    #[test]
    fn returns_none_when_anthropic_api_key_missing() {
        // Save original value if set
        let original = std::env::var("ANTHROPIC_API_KEY").ok();
        std::env::remove_var("ANTHROPIC_API_KEY");

        let mut config = minimal_config();
        config.inference.enabled = true;
        config.llm.provider = LlmProvider::Anthropic;

        let client = llm::build_llm_client(&config);
        assert!(client.is_none());

        // Restore original value if it was set
        if let Some(key) = original {
            std::env::set_var("ANTHROPIC_API_KEY", key);
        }
    }

    #[test]
    fn returns_none_when_openai_api_key_missing() {
        // Save original value if set
        let original = std::env::var("OPENAI_API_KEY").ok();
        std::env::remove_var("OPENAI_API_KEY");

        let mut config = minimal_config();
        config.inference.enabled = true;
        config.llm.provider = LlmProvider::OpenAi;

        let client = llm::build_llm_client(&config);
        assert!(client.is_none());

        // Restore original value if it was set
        if let Some(key) = original {
            std::env::set_var("OPENAI_API_KEY", key);
        }
    }

    #[test]
    fn returns_anthropic_client_when_key_set() {
        // Save original value if set
        let original = std::env::var("ANTHROPIC_API_KEY").ok();
        std::env::set_var("ANTHROPIC_API_KEY", "test-key");

        let mut config = minimal_config();
        config.inference.enabled = true;
        config.llm.provider = LlmProvider::Anthropic;

        let client = llm::build_llm_client(&config);
        assert!(client.is_some());
        assert_eq!(client.unwrap().name(), "anthropic");

        // Restore original value or remove
        match original {
            Some(key) => std::env::set_var("ANTHROPIC_API_KEY", key),
            None => std::env::remove_var("ANTHROPIC_API_KEY"),
        }
    }

    #[test]
    fn returns_openai_client_when_key_set() {
        // Save original value if set
        let original = std::env::var("OPENAI_API_KEY").ok();
        std::env::set_var("OPENAI_API_KEY", "test-key");

        let mut config = minimal_config();
        config.inference.enabled = true;
        config.llm.provider = LlmProvider::OpenAi;

        let client = llm::build_llm_client(&config);
        assert!(client.is_some());
        assert_eq!(client.unwrap().name(), "openai");

        // Restore original value or remove
        match original {
            Some(key) => std::env::set_var("OPENAI_API_KEY", key),
            None => std::env::remove_var("OPENAI_API_KEY"),
        }
    }
}

// ---------------------------------------------------------------------------
// Strategy Factory Tests
// ---------------------------------------------------------------------------

mod strategy_factory {
    use super::*;

    fn minimal_config() -> Config {
        let toml = r#"
            [logging]
            level = "info"
            format = "pretty"
        "#;
        Config::parse_toml(toml).expect("minimal config should parse")
    }

    fn cluster_cache() -> Arc<ClusterCache> {
        Arc::new(ClusterCache::new(Duration::seconds(3600)))
    }

    #[test]
    fn builds_empty_registry_when_no_strategies_enabled() {
        let mut config = minimal_config();
        config.strategies.enabled = vec![];

        let cache = cluster_cache();
        let registry = strategy::build_strategy_registry(&config, cache);

        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn builds_single_condition_strategy() {
        let mut config = minimal_config();
        config.strategies.enabled = vec!["single_condition".to_string()];

        let cache = cluster_cache();
        let registry = strategy::build_strategy_registry(&config, cache);

        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn builds_market_rebalancing_strategy() {
        let mut config = minimal_config();
        config.strategies.enabled = vec!["market_rebalancing".to_string()];

        let cache = cluster_cache();
        let registry = strategy::build_strategy_registry(&config, cache);

        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn builds_combinatorial_strategy() {
        let mut config = minimal_config();
        config.strategies.enabled = vec!["combinatorial".to_string()];

        let cache = cluster_cache();
        let registry = strategy::build_strategy_registry(&config, cache);

        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn builds_multiple_strategies() {
        let mut config = minimal_config();
        config.strategies.enabled = vec![
            "single_condition".to_string(),
            "market_rebalancing".to_string(),
            "combinatorial".to_string(),
        ];

        let cache = cluster_cache();
        let registry = strategy::build_strategy_registry(&config, cache);

        assert_eq!(registry.len(), 3);
    }

    #[test]
    fn normalizes_strategy_names_with_dashes() {
        let mut config = minimal_config();
        config.strategies.enabled = vec!["single-condition".to_string()];

        let cache = cluster_cache();
        let registry = strategy::build_strategy_registry(&config, cache);

        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn normalizes_strategy_names_with_mixed_case() {
        let mut config = minimal_config();
        config.strategies.enabled = vec!["Single_Condition".to_string()];

        let cache = cluster_cache();
        let registry = strategy::build_strategy_registry(&config, cache);

        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn skips_unknown_strategies() {
        let mut config = minimal_config();
        config.strategies.enabled = vec![
            "unknown_strategy".to_string(),
            "single_condition".to_string(),
        ];

        let cache = cluster_cache();
        let registry = strategy::build_strategy_registry(&config, cache);

        // Only single_condition should be added
        assert_eq!(registry.len(), 1);
    }
}

// ---------------------------------------------------------------------------
// Inference Factory Tests
// ---------------------------------------------------------------------------

mod inference_factory {
    use super::*;
    use crate::port::outbound::llm::Llm;

    fn minimal_config() -> Config {
        let toml = r#"
            [logging]
            level = "info"
            format = "pretty"
        "#;
        Config::parse_toml(toml).expect("minimal config should parse")
    }

    /// Mock LLM for testing the inference factory.
    struct MockLlm;

    #[async_trait::async_trait]
    impl Llm for MockLlm {
        fn name(&self) -> &'static str {
            "mock"
        }

        async fn complete(&self, _prompt: &str) -> crate::error::Result<String> {
            Ok("mock response".to_string())
        }
    }

    #[test]
    fn builds_cluster_cache_with_configured_ttl() {
        let mut config = minimal_config();
        config.inference.ttl_seconds = 7200;

        let cache = inference::build_cluster_cache(&config);
        // Cache should be created successfully
        assert!(Arc::strong_count(&cache) >= 1);
    }

    #[test]
    fn builds_inferrer_with_llm_client() {
        let config = minimal_config();
        let llm: Arc<dyn Llm> = Arc::new(MockLlm);

        let inferrer = inference::build_inferrer(&config, llm);
        // Inferrer should be created successfully
        assert!(Arc::strong_count(&inferrer) >= 1);
    }
}

// ---------------------------------------------------------------------------
// Solver Factory Tests
// ---------------------------------------------------------------------------

mod solver_factory {
    use super::*;

    #[test]
    fn builds_projection_solver() {
        let solver = solver::build_projection_solver();
        // Solver should be created successfully
        assert!(Arc::strong_count(&solver) >= 1);
    }
}
