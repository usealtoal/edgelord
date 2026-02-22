//! Governor configuration for adaptive subscription management.

use serde::Deserialize;

const fn default_true() -> bool {
    true
}

const fn default_target_p50_ms() -> u64 {
    10
}

const fn default_target_p95_ms() -> u64 {
    50
}

const fn default_target_p99_ms() -> u64 {
    100
}

const fn default_max_p99_ms() -> u64 {
    200
}

const fn default_check_interval_secs() -> u64 {
    10
}

fn default_expand_threshold() -> f64 {
    0.70
}

fn default_contract_threshold() -> f64 {
    1.20
}

const fn default_expand_step() -> usize {
    50
}

const fn default_contract_step() -> usize {
    100
}

const fn default_cooldown_secs() -> u64 {
    60
}

/// Latency target configuration for the governor.
#[derive(Debug, Clone, Deserialize)]
pub struct LatencyTargetsConfig {
    /// Target p50 latency in milliseconds.
    #[serde(default = "default_target_p50_ms")]
    pub target_p50_ms: u64,
    /// Target p95 latency in milliseconds.
    #[serde(default = "default_target_p95_ms")]
    pub target_p95_ms: u64,
    /// Target p99 latency in milliseconds.
    #[serde(default = "default_target_p99_ms")]
    pub target_p99_ms: u64,
    /// Maximum acceptable p99 latency in milliseconds.
    #[serde(default = "default_max_p99_ms")]
    pub max_p99_ms: u64,
}

impl Default for LatencyTargetsConfig {
    fn default() -> Self {
        Self {
            target_p50_ms: default_target_p50_ms(),
            target_p95_ms: default_target_p95_ms(),
            target_p99_ms: default_target_p99_ms(),
            max_p99_ms: default_max_p99_ms(),
        }
    }
}

/// Scaling configuration for the governor.
#[derive(Debug, Clone, Deserialize)]
pub struct ScalingAppConfig {
    /// Interval between scaling checks in seconds.
    #[serde(default = "default_check_interval_secs")]
    pub check_interval_secs: u64,
    /// Utilization threshold below which to expand subscriptions.
    #[serde(default = "default_expand_threshold")]
    pub expand_threshold: f64,
    /// Utilization threshold above which to contract subscriptions.
    #[serde(default = "default_contract_threshold")]
    pub contract_threshold: f64,
    /// Number of subscriptions to add when expanding.
    #[serde(default = "default_expand_step")]
    pub expand_step: usize,
    /// Number of subscriptions to remove when contracting.
    #[serde(default = "default_contract_step")]
    pub contract_step: usize,
    /// Cooldown period between scaling actions in seconds.
    #[serde(default = "default_cooldown_secs")]
    pub cooldown_secs: u64,
}

impl Default for ScalingAppConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: default_check_interval_secs(),
            contract_threshold: default_contract_threshold(),
            expand_threshold: default_expand_threshold(),
            expand_step: default_expand_step(),
            contract_step: default_contract_step(),
            cooldown_secs: default_cooldown_secs(),
        }
    }
}

/// Governor configuration for adaptive subscription management.
#[derive(Debug, Clone, Deserialize)]
pub struct GovernorAppConfig {
    /// Enable the governor for adaptive scaling.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Latency target configuration.
    #[serde(default)]
    pub latency: LatencyTargetsConfig,
    /// Scaling configuration.
    #[serde(default)]
    pub scaling: ScalingAppConfig,
}

impl Default for GovernorAppConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            latency: LatencyTargetsConfig::default(),
            scaling: ScalingAppConfig::default(),
        }
    }
}
