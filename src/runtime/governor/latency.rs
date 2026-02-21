//! Latency-based adaptive governor implementation.
//!
//! Provides a concrete implementation of [`AdaptiveGovernor`] that makes scaling
//! decisions based on observed latency percentiles. The governor maintains sliding
//! windows of latency and throughput samples, computes percentile metrics, and
//! recommends scaling actions when latency exceeds configured thresholds.

use std::collections::VecDeque;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use crate::domain::{ResourceBudget, ScalingRecommendation};

use super::{AdaptiveGovernor, GovernorConfig, LatencyMetrics};

/// Maximum number of latency/throughput samples to retain.
const MAX_SAMPLES: usize = 1000;

/// Latency-based adaptive governor.
///
/// Monitors latency and throughput metrics to make scaling decisions. Uses a
/// sliding window of samples to compute percentile latencies (p50, p95, p99)
/// and compares them against configured thresholds.
///
/// # Thread Safety
///
/// All mutable state is protected by `RwLock` for thread-safe access from
/// multiple tasks.
///
/// # Example
///
/// ```
/// use edgelord::runtime::governor::{GovernorConfig, LatencyGovernor, AdaptiveGovernor};
/// use std::time::Duration;
///
/// let governor = LatencyGovernor::new(GovernorConfig::default());
///
/// // Record some latency samples
/// governor.record_latency(Duration::from_millis(5));
/// governor.record_latency(Duration::from_millis(10));
/// governor.record_latency(Duration::from_millis(15));
///
/// // Get metrics and recommendation
/// let metrics = governor.latency_metrics();
/// let rec = governor.recommendation();
/// ```
pub struct LatencyGovernor {
    /// Governor configuration.
    config: GovernorConfig,
    /// Sliding window of latency samples.
    samples: RwLock<VecDeque<Duration>>,
    /// Sliding window of throughput samples.
    throughput: RwLock<VecDeque<f64>>,
    /// Timestamp of last scaling action (for cooldown).
    last_scaled: RwLock<Option<Instant>>,
    /// Current resource budget.
    budget: RwLock<ResourceBudget>,
    /// Maximum number of samples to retain.
    max_samples: usize,
}

impl LatencyGovernor {
    /// Create a new latency governor with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - The governor configuration
    ///
    /// # Example
    ///
    /// ```
    /// use edgelord::runtime::governor::{GovernorConfig, LatencyGovernor};
    ///
    /// let governor = LatencyGovernor::new(GovernorConfig::default());
    /// ```
    #[must_use]
    pub fn new(config: GovernorConfig) -> Self {
        Self {
            config,
            samples: RwLock::new(VecDeque::with_capacity(MAX_SAMPLES)),
            throughput: RwLock::new(VecDeque::with_capacity(MAX_SAMPLES)),
            last_scaled: RwLock::new(None),
            budget: RwLock::new(ResourceBudget::default()),
            max_samples: MAX_SAMPLES,
        }
    }

    /// Check if the governor is currently in cooldown period.
    ///
    /// Returns `true` if a scaling action was performed recently and the
    /// cooldown period has not yet elapsed.
    fn in_cooldown(&self) -> bool {
        let last_scaled = self.last_scaled.read().expect("lock poisoned");
        if let Some(instant) = *last_scaled {
            instant.elapsed() < self.config.scaling.cooldown
        } else {
            false
        }
    }

    /// Compute a percentile value from a sorted slice of durations.
    ///
    /// # Arguments
    ///
    /// * `samples` - A sorted slice of duration samples
    /// * `p` - The percentile to compute (0.0 to 1.0)
    ///
    /// # Returns
    ///
    /// The duration at the given percentile, or `Duration::ZERO` if samples is empty.
    fn percentile(samples: &[Duration], p: f64) -> Duration {
        if samples.is_empty() {
            return Duration::ZERO;
        }

        let index = ((samples.len() as f64 - 1.0) * p).round() as usize;
        samples[index.min(samples.len() - 1)]
    }
}

impl AdaptiveGovernor for LatencyGovernor {
    fn record_latency(&self, latency: Duration) {
        let mut samples = self.samples.write().expect("lock poisoned");
        samples.push_back(latency);

        // Trim to max samples
        while samples.len() > self.max_samples {
            samples.pop_front();
        }
    }

    fn record_throughput(&self, messages_per_sec: f64) {
        let mut throughput = self.throughput.write().expect("lock poisoned");
        throughput.push_back(messages_per_sec);

        // Trim to max samples
        while throughput.len() > self.max_samples {
            throughput.pop_front();
        }
    }

    fn latency_metrics(&self) -> LatencyMetrics {
        let samples = self.samples.read().expect("lock poisoned");

        if samples.is_empty() {
            return LatencyMetrics::default();
        }

        // Sort samples for percentile calculation
        let mut sorted: Vec<Duration> = samples.iter().copied().collect();
        sorted.sort();

        LatencyMetrics {
            p50: Self::percentile(&sorted, 0.50),
            p95: Self::percentile(&sorted, 0.95),
            p99: Self::percentile(&sorted, 0.99),
            sample_count: samples.len(),
        }
    }

    fn recommendation(&self) -> ScalingRecommendation {
        // If governor is disabled, always hold
        if !self.config.enabled {
            return ScalingRecommendation::Hold;
        }

        // If in cooldown, hold
        if self.in_cooldown() {
            return ScalingRecommendation::Hold;
        }

        let metrics = self.latency_metrics();

        // Need at least some samples to make a decision
        if metrics.sample_count == 0 {
            return ScalingRecommendation::Hold;
        }

        let target_p95 = self.config.latency.target_p95;
        let max_p99 = self.config.latency.max_p99;

        // Calculate p95 utilization ratio
        let p95_ratio = metrics.p95.as_secs_f64() / target_p95.as_secs_f64();

        // If p99 exceeds max, contract immediately
        if metrics.p99 > max_p99 {
            let budget = self.budget.read().expect("lock poisoned");
            let max_subs = budget.estimate_max_subscriptions();
            let contract_to = max_subs.saturating_sub(self.config.scaling.contract_step);
            return ScalingRecommendation::contract(contract_to);
        }

        // If p95 exceeds contract threshold, contract
        if p95_ratio > self.config.scaling.contract_threshold {
            let budget = self.budget.read().expect("lock poisoned");
            let max_subs = budget.estimate_max_subscriptions();
            let contract_to = max_subs.saturating_sub(self.config.scaling.contract_step);
            return ScalingRecommendation::contract(contract_to);
        }

        // If p95 is below expand threshold (with hysteresis), expand
        let expand_threshold_with_hysteresis =
            self.config.scaling.expand_threshold - self.config.scaling.hysteresis;
        if p95_ratio < expand_threshold_with_hysteresis {
            let budget = self.budget.read().expect("lock poisoned");
            let max_subs = budget.estimate_max_subscriptions();
            let expand_to = max_subs + self.config.scaling.expand_step;
            return ScalingRecommendation::expand(expand_to);
        }

        ScalingRecommendation::Hold
    }

    fn notify_scaled(&self) {
        let mut last_scaled = self.last_scaled.write().expect("lock poisoned");
        *last_scaled = Some(Instant::now());
    }

    fn set_resource_budget(&self, budget: ResourceBudget) {
        let mut current_budget = self.budget.write().expect("lock poisoned");
        *current_budget = budget;
    }

    fn config(&self) -> &GovernorConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::super::{LatencyTargets, ScalingConfig};
    use super::*;

    // --- LatencyGovernor::new tests ---

    #[test]
    fn latency_governor_new_creates_empty_samples() {
        let governor = LatencyGovernor::new(GovernorConfig::default());

        let samples = governor.samples.read().unwrap();
        assert!(samples.is_empty());
    }

    #[test]
    fn latency_governor_new_creates_empty_throughput() {
        let governor = LatencyGovernor::new(GovernorConfig::default());

        let throughput = governor.throughput.read().unwrap();
        assert!(throughput.is_empty());
    }

    #[test]
    fn latency_governor_new_has_no_last_scaled() {
        let governor = LatencyGovernor::new(GovernorConfig::default());

        let last_scaled = governor.last_scaled.read().unwrap();
        assert!(last_scaled.is_none());
    }

    #[test]
    fn latency_governor_new_has_default_budget() {
        let governor = LatencyGovernor::new(GovernorConfig::default());

        let budget = governor.budget.read().unwrap();
        assert_eq!(*budget, ResourceBudget::default());
    }

    #[test]
    fn latency_governor_new_sets_max_samples() {
        let governor = LatencyGovernor::new(GovernorConfig::default());

        assert_eq!(governor.max_samples, MAX_SAMPLES);
    }

    // --- LatencyGovernor::record_latency tests ---

    #[test]
    fn latency_governor_record_latency_adds_sample() {
        let governor = LatencyGovernor::new(GovernorConfig::default());

        governor.record_latency(Duration::from_millis(10));

        let samples = governor.samples.read().unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0], Duration::from_millis(10));
    }

    #[test]
    fn latency_governor_record_latency_multiple_samples() {
        let governor = LatencyGovernor::new(GovernorConfig::default());

        governor.record_latency(Duration::from_millis(10));
        governor.record_latency(Duration::from_millis(20));
        governor.record_latency(Duration::from_millis(30));

        let samples = governor.samples.read().unwrap();
        assert_eq!(samples.len(), 3);
    }

    #[test]
    fn latency_governor_record_latency_trims_to_max() {
        let config = GovernorConfig::default();
        let governor = LatencyGovernor::new(config);

        // Record more samples than max_samples
        for i in 0..=MAX_SAMPLES {
            governor.record_latency(Duration::from_millis(i as u64));
        }

        let samples = governor.samples.read().unwrap();
        assert_eq!(samples.len(), MAX_SAMPLES);

        // First sample should have been removed
        assert_eq!(samples[0], Duration::from_millis(1));
    }

    // --- LatencyGovernor::record_throughput tests ---

    #[test]
    fn latency_governor_record_throughput_adds_sample() {
        let governor = LatencyGovernor::new(GovernorConfig::default());

        governor.record_throughput(100.0);

        let throughput = governor.throughput.read().unwrap();
        assert_eq!(throughput.len(), 1);
        assert!((throughput[0] - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn latency_governor_record_throughput_trims_to_max() {
        let governor = LatencyGovernor::new(GovernorConfig::default());

        for i in 0..=MAX_SAMPLES {
            governor.record_throughput(i as f64);
        }

        let throughput = governor.throughput.read().unwrap();
        assert_eq!(throughput.len(), MAX_SAMPLES);
    }

    // --- LatencyGovernor::latency_metrics tests ---

    #[test]
    fn latency_governor_latency_metrics_empty_returns_default() {
        let governor = LatencyGovernor::new(GovernorConfig::default());

        let metrics = governor.latency_metrics();

        assert_eq!(metrics, LatencyMetrics::default());
    }

    #[test]
    fn latency_governor_latency_metrics_single_sample() {
        let governor = LatencyGovernor::new(GovernorConfig::default());
        governor.record_latency(Duration::from_millis(50));

        let metrics = governor.latency_metrics();

        assert_eq!(metrics.p50, Duration::from_millis(50));
        assert_eq!(metrics.p95, Duration::from_millis(50));
        assert_eq!(metrics.p99, Duration::from_millis(50));
        assert_eq!(metrics.sample_count, 1);
    }

    #[test]
    fn latency_governor_latency_metrics_computes_percentiles() {
        let governor = LatencyGovernor::new(GovernorConfig::default());

        // Add 100 samples: 1ms, 2ms, ..., 100ms
        for i in 1..=100 {
            governor.record_latency(Duration::from_millis(i));
        }

        let metrics = governor.latency_metrics();

        // p50 should be around 50ms (rounding may give 50 or 51)
        assert!(
            metrics.p50 >= Duration::from_millis(49) && metrics.p50 <= Duration::from_millis(51)
        );
        // p95 should be around 95ms (rounding may give 94, 95, or 96)
        assert!(
            metrics.p95 >= Duration::from_millis(94) && metrics.p95 <= Duration::from_millis(96)
        );
        // p99 should be around 99ms
        assert!(
            metrics.p99 >= Duration::from_millis(98) && metrics.p99 <= Duration::from_millis(100)
        );
        assert_eq!(metrics.sample_count, 100);
    }

    // --- LatencyGovernor::percentile tests ---

    #[test]
    fn latency_governor_percentile_empty_returns_zero() {
        let samples: Vec<Duration> = vec![];
        let p50 = LatencyGovernor::percentile(&samples, 0.50);

        assert_eq!(p50, Duration::ZERO);
    }

    #[test]
    fn latency_governor_percentile_single_element() {
        let samples = vec![Duration::from_millis(10)];

        assert_eq!(
            LatencyGovernor::percentile(&samples, 0.0),
            Duration::from_millis(10)
        );
        assert_eq!(
            LatencyGovernor::percentile(&samples, 0.5),
            Duration::from_millis(10)
        );
        assert_eq!(
            LatencyGovernor::percentile(&samples, 1.0),
            Duration::from_millis(10)
        );
    }

    #[test]
    fn latency_governor_percentile_sorted_input() {
        let samples: Vec<Duration> = (1..=10).map(|i| Duration::from_millis(i * 10)).collect();

        // p0 = 10ms (index 0)
        assert_eq!(
            LatencyGovernor::percentile(&samples, 0.0),
            Duration::from_millis(10)
        );
        // p50 = 50ms (index 4.5 -> rounds to 5 -> 60ms? Actually (9 * 0.5).round() = 4)
        // Index = (10 - 1) * 0.5 = 4.5, rounded = 4 or 5
        let p50 = LatencyGovernor::percentile(&samples, 0.5);
        assert!(p50 == Duration::from_millis(50) || p50 == Duration::from_millis(60));
        // p100 = 100ms (index 9)
        assert_eq!(
            LatencyGovernor::percentile(&samples, 1.0),
            Duration::from_millis(100)
        );
    }

    // --- LatencyGovernor::in_cooldown tests ---

    #[test]
    fn latency_governor_in_cooldown_false_initially() {
        let governor = LatencyGovernor::new(GovernorConfig::default());

        assert!(!governor.in_cooldown());
    }

    #[test]
    fn latency_governor_in_cooldown_true_after_notify_scaled() {
        let governor = LatencyGovernor::new(GovernorConfig::default());
        governor.notify_scaled();

        assert!(governor.in_cooldown());
    }

    // --- LatencyGovernor::notify_scaled tests ---

    #[test]
    fn latency_governor_notify_scaled_sets_timestamp() {
        let governor = LatencyGovernor::new(GovernorConfig::default());

        let before = Instant::now();
        governor.notify_scaled();
        let after = Instant::now();

        let last_scaled = governor.last_scaled.read().unwrap();
        assert!(last_scaled.is_some());
        let instant = last_scaled.unwrap();
        assert!(instant >= before && instant <= after);
    }

    // --- LatencyGovernor::set_resource_budget tests ---

    #[test]
    fn latency_governor_set_resource_budget_updates_budget() {
        let governor = LatencyGovernor::new(GovernorConfig::default());
        let new_budget = ResourceBudget::production();

        governor.set_resource_budget(new_budget);

        let budget = governor.budget.read().unwrap();
        assert_eq!(*budget, ResourceBudget::production());
    }

    // --- LatencyGovernor::config tests ---

    #[test]
    fn latency_governor_config_returns_config() {
        let config = GovernorConfig {
            enabled: false,
            ..Default::default()
        };
        let governor = LatencyGovernor::new(config.clone());

        assert_eq!(governor.config(), &config);
    }

    // --- LatencyGovernor::recommendation tests ---

    #[test]
    fn latency_governor_recommendation_hold_when_disabled() {
        let config = GovernorConfig {
            enabled: false,
            ..Default::default()
        };
        let governor = LatencyGovernor::new(config);

        // Add samples that would trigger scaling
        for _ in 0..100 {
            governor.record_latency(Duration::from_millis(200));
        }

        let rec = governor.recommendation();
        assert!(rec.is_hold());
    }

    #[test]
    fn latency_governor_recommendation_hold_when_no_samples() {
        let governor = LatencyGovernor::new(GovernorConfig::default());

        let rec = governor.recommendation();
        assert!(rec.is_hold());
    }

    #[test]
    fn latency_governor_recommendation_hold_in_cooldown() {
        let governor = LatencyGovernor::new(GovernorConfig::default());

        // Add samples
        for _ in 0..100 {
            governor.record_latency(Duration::from_millis(1));
        }

        // Trigger cooldown
        governor.notify_scaled();

        let rec = governor.recommendation();
        assert!(rec.is_hold());
    }

    #[test]
    fn latency_governor_recommendation_contract_when_p99_exceeds_max() {
        let config = GovernorConfig {
            latency: LatencyTargets {
                target_p95: Duration::from_millis(50),
                max_p99: Duration::from_millis(100),
                ..Default::default()
            },
            ..Default::default()
        };
        let governor = LatencyGovernor::new(config);

        // Add samples with high latency
        for i in 1..=100 {
            // p99 will be very high
            governor.record_latency(Duration::from_millis(100 + i * 10));
        }

        let rec = governor.recommendation();
        assert!(rec.is_contract());
    }

    #[test]
    fn latency_governor_recommendation_contract_when_p95_high() {
        let config = GovernorConfig {
            latency: LatencyTargets {
                target_p95: Duration::from_millis(50),
                max_p99: Duration::from_millis(500),
                ..Default::default()
            },
            scaling: ScalingConfig {
                contract_threshold: 0.9, // contract when p95 > 45ms (90% of 50ms)
                ..Default::default()
            },
            ..Default::default()
        };
        let governor = LatencyGovernor::new(config);

        // Add samples where p95 will be ~95ms (190% of target)
        for i in 1..=100 {
            governor.record_latency(Duration::from_millis(i));
        }

        let rec = governor.recommendation();
        assert!(rec.is_contract());
    }

    #[test]
    fn latency_governor_recommendation_expand_when_latency_low() {
        let config = GovernorConfig {
            latency: LatencyTargets {
                target_p95: Duration::from_millis(100),
                max_p99: Duration::from_millis(500),
                ..Default::default()
            },
            scaling: ScalingConfig {
                expand_threshold: 0.6, // expand when p95 < 60ms (60% of 100ms)
                hysteresis: 0.1,       // with hysteresis: expand when < 50ms
                ..Default::default()
            },
            ..Default::default()
        };
        let governor = LatencyGovernor::new(config);

        // Add low latency samples (p95 will be ~19ms, well below threshold)
        for i in 1..=100 {
            governor.record_latency(Duration::from_millis(i / 5)); // 0-20ms range
        }

        let rec = governor.recommendation();
        assert!(rec.is_expand());
    }

    #[test]
    fn latency_governor_recommendation_hold_in_middle_range() {
        let config = GovernorConfig {
            latency: LatencyTargets {
                target_p95: Duration::from_millis(100),
                max_p99: Duration::from_millis(500),
                ..Default::default()
            },
            scaling: ScalingConfig {
                expand_threshold: 0.6,   // expand when < 60ms
                contract_threshold: 0.9, // contract when > 90ms
                hysteresis: 0.1,         // expand threshold becomes 50ms
                ..Default::default()
            },
            ..Default::default()
        };
        let governor = LatencyGovernor::new(config);

        // Add samples where p95 will be ~70ms (in middle range 50-90ms)
        // We want p95 to be about 70ms, so we add mostly samples around that range
        for _ in 0..100 {
            governor.record_latency(Duration::from_millis(70));
        }

        let rec = governor.recommendation();
        assert!(rec.is_hold());
    }

    // --- Thread safety test ---

    #[test]
    fn latency_governor_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<LatencyGovernor>();
    }
}
