//! Inference service configuration.

use serde::Deserialize;

/// Inference service configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct InferenceConfig {
    /// Enable inference service.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Minimum confidence threshold (0.0-1.0).
    #[serde(default = "default_min_confidence")]
    pub min_confidence: f64,
    /// Relation TTL in seconds.
    #[serde(default = "default_ttl_seconds")]
    pub ttl_seconds: u64,
    /// Price change threshold to trigger re-inference (0.0-1.0).
    #[serde(default = "default_price_threshold")]
    pub price_change_threshold: f64,
    /// Full scan interval in seconds.
    #[serde(default = "default_scan_interval")]
    pub scan_interval_seconds: u64,
    /// Maximum markets per inference batch.
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            min_confidence: default_min_confidence(),
            ttl_seconds: default_ttl_seconds(),
            price_change_threshold: default_price_threshold(),
            scan_interval_seconds: default_scan_interval(),
            batch_size: default_batch_size(),
        }
    }
}

const fn default_enabled() -> bool {
    true
}

const fn default_min_confidence() -> f64 {
    0.7
}

const fn default_ttl_seconds() -> u64 {
    3600
}

const fn default_price_threshold() -> f64 {
    0.05
}

const fn default_scan_interval() -> u64 {
    3600
}

const fn default_batch_size() -> usize {
    30
}
