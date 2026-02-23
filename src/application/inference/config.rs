//! Inference service configuration.

use serde::Deserialize;

/// Configuration for the relation inference service.
#[derive(Debug, Clone, Deserialize)]
pub struct InferenceConfig {
    /// Whether the inference service is enabled.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Minimum confidence threshold for accepting inferred relations (0.0 to 1.0).
    #[serde(default = "default_min_confidence")]
    pub min_confidence: f64,
    /// Time-to-live for cached relations in seconds.
    #[serde(default = "default_ttl_seconds")]
    pub ttl_seconds: u64,
    /// Price change threshold that triggers re-inference (0.0 to 1.0).
    #[serde(default = "default_price_threshold")]
    pub price_change_threshold: f64,
    /// Interval between full market scans in seconds.
    #[serde(default = "default_scan_interval")]
    pub scan_interval_seconds: u64,
    /// Maximum number of markets to process per inference batch.
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
    50
}
