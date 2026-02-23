//! Cluster detection service configuration.
//!
//! Provides configuration for the cluster-based arbitrage detection service
//! that monitors related markets for pricing inefficiencies.

use rust_decimal::Decimal;
use serde::Deserialize;

use crate::application::cluster::service::ClusterDetectionConfig as CoreConfig;

/// Configuration for the cluster detection service.
///
/// Controls the behavior of the background service that detects arbitrage
/// opportunities across clusters of related markets.
#[derive(Debug, Clone, Deserialize)]
pub struct ClusterDetectionConfig {
    /// Enable the cluster detection service.
    ///
    /// When false, cluster-based arbitrage detection is disabled entirely.
    /// Defaults to false.
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Debounce interval in milliseconds.
    ///
    /// Minimum time between processing the same cluster after an order book
    /// update. Prevents excessive CPU usage from rapid updates.
    /// Defaults to 100ms.
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,

    /// Minimum arbitrage gap to report an opportunity.
    ///
    /// Expressed as a decimal fraction (e.g., 0.02 for 2%).
    /// Opportunities below this threshold are ignored.
    /// Defaults to 0.02 (2%).
    #[serde(default = "default_min_gap")]
    pub min_gap: Decimal,

    /// Maximum clusters to process per detection cycle.
    ///
    /// Limits CPU usage by capping how many clusters are evaluated
    /// in a single pass. Defaults to 50.
    #[serde(default = "default_max_clusters_per_cycle")]
    pub max_clusters_per_cycle: usize,

    /// Channel capacity for order book update notifications.
    ///
    /// Size of the bounded channel that buffers update notifications.
    /// Larger values reduce backpressure but increase memory usage.
    /// Defaults to 1024.
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
    /// Convert to the core service configuration type.
    ///
    /// Creates a [`CoreConfig`] suitable for initializing the cluster
    /// detection service.
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
