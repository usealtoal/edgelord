//! Cluster detection service configuration.

use rust_decimal::Decimal;
use serde::Deserialize;

use crate::adapters::cluster::ClusterDetectionConfig as CoreConfig;

/// Configuration for the cluster detection service.
#[derive(Debug, Clone, Deserialize)]
pub struct ClusterDetectionConfig {
    /// Enable cluster detection service.
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Debounce interval in milliseconds.
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,

    /// Minimum arbitrage gap to report an opportunity.
    #[serde(default = "default_min_gap")]
    pub min_gap: Decimal,

    /// Maximum clusters to process per detection cycle.
    #[serde(default = "default_max_clusters_per_cycle")]
    pub max_clusters_per_cycle: usize,

    /// Channel capacity for order book update notifications.
    #[serde(default = "default_channel_capacity")]
    pub channel_capacity: usize,
}

impl Default for ClusterDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            debounce_ms: default_debounce_ms(),
            min_gap: default_min_gap(),
            max_clusters_per_cycle: default_max_clusters_per_cycle(),
            channel_capacity: default_channel_capacity(),
        }
    }
}

impl ClusterDetectionConfig {
    /// Convert to core service config.
    #[must_use]
    pub fn to_core_config(&self) -> CoreConfig {
        CoreConfig {
            debounce_ms: self.debounce_ms,
            min_gap: self.min_gap,
            max_clusters_per_cycle: self.max_clusters_per_cycle,
        }
    }
}

const fn default_enabled() -> bool {
    false
}

const fn default_debounce_ms() -> u64 {
    100
}

fn default_min_gap() -> Decimal {
    Decimal::new(2, 2) // 0.02
}

const fn default_max_clusters_per_cycle() -> usize {
    50
}

const fn default_channel_capacity() -> usize {
    1024
}
