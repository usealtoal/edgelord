//! Infrastructure bootstrap helpers for runtime wiring.
//!
//! Provides a thin composition root that re-exports factory functions for
//! building infrastructure components. This module serves as a convenience
//! layer for application startup.
//!
//! Consumers can use either `bootstrap::build_*` or `factory::*::build_*`
//! depending on preference.

// Re-export factory functions for backwards compatibility and convenience.

pub use crate::infrastructure::factory::executor::build_executor as init_executor;
pub use crate::infrastructure::factory::inference::{build_cluster_cache, build_inferrer};
pub use crate::infrastructure::factory::llm::build_llm_client;
pub use crate::infrastructure::factory::notifier::build_notifier_registry;
pub use crate::infrastructure::factory::persistence::build_stats_recorder as init_stats_recorder;
pub use crate::infrastructure::factory::solver::build_projection_solver;
pub use crate::infrastructure::factory::strategy::build_strategy_registry;

#[cfg(test)]
mod tests {
    //! Tests for bootstrap module re-exports.
    //!
    //! Verifies that the bootstrap module correctly re-exports factory functions
    //! and that they are accessible via the bootstrap namespace.

    use super::*;
    use std::sync::Arc;

    use chrono::Duration;

    use crate::application::cache::cluster::ClusterCache;
    use crate::infrastructure::config::settings::Config;

    fn minimal_config() -> Config {
        let toml = r#"
            [logging]
            level = "info"
            format = "pretty"
        "#;
        Config::parse_toml(toml).expect("minimal config should parse")
    }

    #[test]
    fn build_llm_client_re_export_works() {
        let mut config = minimal_config();
        config.inference.enabled = false;

        // Should return None when inference disabled
        let client = build_llm_client(&config);
        assert!(client.is_none());
    }

    #[test]
    fn build_cluster_cache_re_export_works() {
        let config = minimal_config();
        let cache = build_cluster_cache(&config);
        assert!(Arc::strong_count(&cache) >= 1);
    }

    #[test]
    fn build_projection_solver_re_export_works() {
        let solver = build_projection_solver();
        assert!(Arc::strong_count(&solver) >= 1);
    }

    #[test]
    fn build_strategy_registry_re_export_works() {
        let mut config = minimal_config();
        config.strategies.enabled = vec!["single_condition".to_string()];

        let cache = Arc::new(ClusterCache::new(Duration::seconds(3600)));
        let registry = build_strategy_registry(&config, cache);

        assert_eq!(registry.len(), 1);
    }
}
