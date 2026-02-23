//! Inference factory for relation detection services.
//!
//! Provides factory functions for constructing inference-related components
//! including the cluster cache and LLM-based relation inferrer.

use std::sync::Arc;

use chrono::Duration;

use crate::adapter::outbound::inference::inferrer::LlmInferrer;
use crate::application::cache::cluster::ClusterCache;
use crate::infrastructure::config::settings::Config;
use crate::port::outbound::inference::RelationInferrer;
use crate::port::outbound::llm::Llm;

/// Build the cluster cache for relation inference.
///
/// Creates a cache with TTL configured from the inference settings.
/// The cache stores discovered market clusters to avoid redundant inference.
pub fn build_cluster_cache(config: &Config) -> Arc<ClusterCache> {
    let ttl = Duration::seconds(config.inference.ttl_seconds as i64);
    Arc::new(ClusterCache::new(ttl))
}

/// Build the inference service adapter.
///
/// Creates an LLM-based inferrer that detects relationships between markets.
/// Requires a configured LLM client.
pub fn build_inferrer(config: &Config, llm: Arc<dyn Llm>) -> Arc<dyn RelationInferrer> {
    let ttl = Duration::seconds(config.inference.ttl_seconds as i64);
    Arc::new(LlmInferrer::new(llm, ttl))
}
