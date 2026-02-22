//! Adaptive governor for monitoring performance and signaling scaling decisions.
//!
//! The [`AdaptiveGovernor`] trait provides the interface for monitoring latency and
//! throughput metrics, then recommending scaling actions based on configurable targets
//! and thresholds.

mod latency;
pub use latency::LatencyGovernor;

use std::time::Duration;

use super::{ResourceBudget, ScalingRecommendation};

/// Latency targets for the adaptive governor.
///
/// Defines the latency percentile targets that the governor uses to determine
/// when the system is performing well or needs to scale.
///
/// # Example
///
/// ```
/// use edgelord::infrastructure::governor::LatencyTargets;
/// use std::time::Duration;
///
/// let targets = LatencyTargets {
///     target_p50: Duration::from_millis(10),
///     target_p95: Duration::from_millis(50),
///     target_p99: Duration::from_millis(100),
///     max_p99: Duration::from_millis(500),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatencyTargets {
    /// Target p50 latency (median).
    pub target_p50: Duration,
    /// Target p95 latency.
    pub target_p95: Duration,
    /// Target p99 latency.
    pub target_p99: Duration,
    /// Maximum acceptable p99 latency before forcing contraction.
    pub max_p99: Duration,
}

impl Default for LatencyTargets {
    fn default() -> Self {
        Self {
            target_p50: Duration::from_millis(10),
            target_p95: Duration::from_millis(50),
            target_p99: Duration::from_millis(100),
            max_p99: Duration::from_millis(500),
        }
    }
}

/// Scaling configuration for the adaptive governor.
///
/// Defines the thresholds and step sizes used when making scaling decisions.
///
/// # Example
///
/// ```
/// use edgelord::infrastructure::governor::ScalingConfig;
/// use std::time::Duration;
///
/// let config = ScalingConfig {
///     check_interval: Duration::from_secs(5),
///     expand_threshold: 0.6,
///     contract_threshold: 0.9,
///     expand_step: 10,
///     contract_step: 5,
///     cooldown: Duration::from_secs(30),
///     hysteresis: 0.1,
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ScalingConfig {
    /// How often to check metrics and make scaling decisions.
    pub check_interval: Duration,
    /// Latency utilization threshold below which expansion is considered.
    /// Value between 0.0 and 1.0 (e.g., 0.6 = expand when at 60% of target latency).
    pub expand_threshold: f64,
    /// Latency utilization threshold above which contraction is triggered.
    /// Value between 0.0 and 1.0 (e.g., 0.9 = contract when at 90% of target latency).
    pub contract_threshold: f64,
    /// Number of subscriptions to add when expanding.
    pub expand_step: usize,
    /// Number of subscriptions to remove when contracting.
    pub contract_step: usize,
    /// Minimum time to wait after a scaling action before scaling again.
    pub cooldown: Duration,
    /// Hysteresis buffer to prevent oscillation (e.g., 0.1 = 10% buffer).
    pub hysteresis: f64,
}

impl Default for ScalingConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(5),
            expand_threshold: 0.6,
            contract_threshold: 0.9,
            expand_step: 10,
            contract_step: 5,
            cooldown: Duration::from_secs(30),
            hysteresis: 0.1,
        }
    }
}

/// Governor configuration combining latency targets and scaling parameters.
///
/// # Example
///
/// ```
/// use edgelord::infrastructure::governor::GovernorConfig;
///
/// // Use defaults
/// let config = GovernorConfig::default();
/// assert!(config.enabled);
///
/// // Disable governor
/// let config = GovernorConfig {
///     enabled: false,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct GovernorConfig {
    /// Whether the adaptive governor is enabled.
    pub enabled: bool,
    /// Latency targets for scaling decisions.
    pub latency: LatencyTargets,
    /// Scaling parameters.
    pub scaling: ScalingConfig,
}

impl Default for GovernorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            latency: LatencyTargets::default(),
            scaling: ScalingConfig::default(),
        }
    }
}

/// Observed latency metrics from the governor.
///
/// Contains the computed percentile values and sample count for the current
/// observation window.
///
/// # Example
///
/// ```
/// use edgelord::infrastructure::governor::LatencyMetrics;
/// use std::time::Duration;
///
/// let metrics = LatencyMetrics {
///     p50: Duration::from_millis(8),
///     p95: Duration::from_millis(35),
///     p99: Duration::from_millis(72),
///     sample_count: 1000,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatencyMetrics {
    /// Observed p50 (median) latency.
    pub p50: Duration,
    /// Observed p95 latency.
    pub p95: Duration,
    /// Observed p99 latency.
    pub p99: Duration,
    /// Number of samples in the current observation window.
    pub sample_count: usize,
}

impl Default for LatencyMetrics {
    fn default() -> Self {
        Self {
            p50: Duration::ZERO,
            p95: Duration::ZERO,
            p99: Duration::ZERO,
            sample_count: 0,
        }
    }
}

/// Monitors performance metrics and signals scaling decisions.
///
/// Implementations track latency and throughput metrics, compare them against
/// configured targets, and produce scaling recommendations. The governor operates
/// synchronously - it records metrics and produces recommendations without blocking.
///
/// # Example
///
/// ```ignore
/// struct MyGovernor {
///     config: GovernorConfig,
///     metrics: Mutex<LatencyWindow>,
/// }
///
/// impl AdaptiveGovernor for MyGovernor {
///     fn record_latency(&self, latency: Duration) {
///         self.metrics.lock().unwrap().record(latency);
///     }
///
///     fn record_throughput(&self, messages_per_sec: f64) {
///         // Track throughput
///     }
///
///     fn latency_metrics(&self) -> LatencyMetrics {
///         self.metrics.lock().unwrap().compute_percentiles()
///     }
///
///     fn recommendation(&self) -> ScalingRecommendation {
///         let metrics = self.latency_metrics();
///         if metrics.p99 > self.config.latency.max_p99 {
///             ScalingRecommendation::contract(self.config.scaling.contract_step)
///         } else {
///             ScalingRecommendation::Hold
///         }
///     }
///
///     fn notify_scaled(&self) {
///         // Reset cooldown timer
///     }
///
///     fn set_resource_budget(&self, budget: ResourceBudget) {
///         // Update resource constraints
///     }
///
///     fn config(&self) -> &GovernorConfig {
///         &self.config
///     }
/// }
/// ```
pub trait AdaptiveGovernor: Send + Sync {
    /// Record a latency observation.
    ///
    /// Should be called for each message processing latency measurement.
    /// Implementations typically maintain a sliding window of observations.
    ///
    /// # Arguments
    ///
    /// * `latency` - The observed latency duration
    fn record_latency(&self, latency: Duration);

    /// Record a throughput observation.
    ///
    /// Should be called periodically with the current throughput rate.
    /// Used in conjunction with latency to inform scaling decisions.
    ///
    /// # Arguments
    ///
    /// * `messages_per_sec` - The observed throughput in messages per second
    fn record_throughput(&self, messages_per_sec: f64);

    /// Get the current latency metrics.
    ///
    /// Returns computed percentile values from the current observation window.
    ///
    /// # Returns
    ///
    /// The current [`LatencyMetrics`] with p50, p95, p99, and sample count.
    fn latency_metrics(&self) -> LatencyMetrics;

    /// Get the current scaling recommendation.
    ///
    /// Analyzes the current metrics against configured targets and thresholds
    /// to produce a scaling recommendation. This method should respect cooldown
    /// periods and hysteresis to prevent oscillation.
    ///
    /// # Returns
    ///
    /// A [`ScalingRecommendation`] indicating whether to expand, hold, or contract.
    fn recommendation(&self) -> ScalingRecommendation;

    /// Notify the governor that a scaling action was performed.
    ///
    /// Should be called after the subscription manager acts on a recommendation.
    /// Implementations typically reset cooldown timers and clear metrics windows.
    fn notify_scaled(&self);

    /// Update the resource budget constraints.
    ///
    /// Allows dynamic adjustment of resource limits, which may affect scaling
    /// decisions (e.g., maximum subscription count based on memory).
    ///
    /// # Arguments
    ///
    /// * `budget` - The new resource budget constraints
    fn set_resource_budget(&self, budget: ResourceBudget);

    /// Get the governor configuration.
    ///
    /// # Returns
    ///
    /// A reference to the [`GovernorConfig`] used by this governor.
    fn config(&self) -> &GovernorConfig;
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- LatencyTargets tests ---

    #[test]
    fn latency_targets_default_has_sensible_values() {
        let targets = LatencyTargets::default();

        assert_eq!(targets.target_p50, Duration::from_millis(10));
        assert_eq!(targets.target_p95, Duration::from_millis(50));
        assert_eq!(targets.target_p99, Duration::from_millis(100));
        assert_eq!(targets.max_p99, Duration::from_millis(500));
    }

    #[test]
    fn latency_targets_clone() {
        let targets = LatencyTargets::default();
        let cloned = targets.clone();

        assert_eq!(targets, cloned);
    }

    // --- ScalingConfig tests ---

    #[test]
    fn scaling_config_default_has_sensible_values() {
        let config = ScalingConfig::default();

        assert_eq!(config.check_interval, Duration::from_secs(5));
        assert!((config.expand_threshold - 0.6).abs() < f64::EPSILON);
        assert!((config.contract_threshold - 0.9).abs() < f64::EPSILON);
        assert_eq!(config.expand_step, 10);
        assert_eq!(config.contract_step, 5);
        assert_eq!(config.cooldown, Duration::from_secs(30));
        assert!((config.hysteresis - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn scaling_config_clone() {
        let config = ScalingConfig::default();
        let cloned = config.clone();

        assert_eq!(config, cloned);
    }

    // --- GovernorConfig tests ---

    #[test]
    fn governor_config_default_is_enabled() {
        let config = GovernorConfig::default();

        assert!(config.enabled);
    }

    #[test]
    fn governor_config_default_has_default_latency_targets() {
        let config = GovernorConfig::default();

        assert_eq!(config.latency, LatencyTargets::default());
    }

    #[test]
    fn governor_config_default_has_default_scaling_config() {
        let config = GovernorConfig::default();

        assert_eq!(config.scaling, ScalingConfig::default());
    }

    #[test]
    fn governor_config_clone() {
        let config = GovernorConfig::default();
        let cloned = config.clone();

        assert_eq!(config, cloned);
    }

    // --- LatencyMetrics tests ---

    #[test]
    fn latency_metrics_default_has_zero_values() {
        let metrics = LatencyMetrics::default();

        assert_eq!(metrics.p50, Duration::ZERO);
        assert_eq!(metrics.p95, Duration::ZERO);
        assert_eq!(metrics.p99, Duration::ZERO);
        assert_eq!(metrics.sample_count, 0);
    }

    #[test]
    fn latency_metrics_clone() {
        let metrics = LatencyMetrics {
            p50: Duration::from_millis(10),
            p95: Duration::from_millis(50),
            p99: Duration::from_millis(100),
            sample_count: 1000,
        };
        let cloned = metrics.clone();

        assert_eq!(metrics, cloned);
    }

    #[test]
    fn latency_metrics_equality() {
        let a = LatencyMetrics {
            p50: Duration::from_millis(10),
            p95: Duration::from_millis(50),
            p99: Duration::from_millis(100),
            sample_count: 1000,
        };
        let b = LatencyMetrics {
            p50: Duration::from_millis(10),
            p95: Duration::from_millis(50),
            p99: Duration::from_millis(100),
            sample_count: 1000,
        };
        let c = LatencyMetrics {
            p50: Duration::from_millis(20),
            p95: Duration::from_millis(50),
            p99: Duration::from_millis(100),
            sample_count: 1000,
        };

        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
