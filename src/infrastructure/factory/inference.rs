//! Inference factory for relation detection services.

use std::sync::Arc;

use chrono::Duration;

use crate::adapter::outbound::inference::inferrer::LlmInferrer;
use crate::application::cache::cluster::ClusterCache;
use crate::infrastructure::config::settings::Config;
use crate::port::outbound::inference::RelationInferrer;
use crate::port::outbound::llm::Llm;

/// Build cluster cache for relation inference.
pub fn build_cluster_cache(config: &Config) -> Arc<ClusterCache> {
    let ttl = Duration::seconds(config.inference.ttl_seconds as i64);
    Arc::new(ClusterCache::new(ttl))
}

/// Build inference service adapter.
pub fn build_inferrer(config: &Config, llm: Arc<dyn Llm>) -> Arc<dyn RelationInferrer> {
    let ttl = Duration::seconds(config.inference.ttl_seconds as i64);
    Arc::new(LlmInferrer::new(llm, ttl))
}
