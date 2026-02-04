//! Resource budget types for adaptive subscription management.
//!
//! These types define resource constraints and targets used for adaptive scaling
//! of market subscriptions based on available system resources.
//!
//! - [`ResourceBudget`] - Configuration for memory and CPU resource limits

/// Memory per subscription estimate in bytes (~10KB).
const BYTES_PER_SUBSCRIPTION: u64 = 10 * 1024;

/// Resource budget configuration for adaptive subscription management.
///
/// Defines the resource constraints and utilization targets that guide
/// how many market subscriptions the system can maintain.
///
/// # Fields
///
/// - `max_memory_bytes` - Maximum memory budget in bytes
/// - `worker_threads` - Number of worker threads available
/// - `memory_target` - Target memory utilization (0.0-1.0)
/// - `cpu_target` - Target CPU utilization (0.0-1.0)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResourceBudget {
    /// Maximum memory budget in bytes.
    pub max_memory_bytes: u64,
    /// Number of worker threads available.
    pub worker_threads: usize,
    /// Target memory utilization (0.0-1.0).
    pub memory_target: f64,
    /// Target CPU utilization (0.0-1.0).
    pub cpu_target: f64,
}

impl ResourceBudget {
    /// Create a new resource budget with explicit values.
    ///
    /// # Arguments
    ///
    /// * `max_memory_bytes` - Maximum memory budget in bytes
    /// * `worker_threads` - Number of worker threads available
    /// * `memory_target` - Target memory utilization (0.0-1.0)
    /// * `cpu_target` - Target CPU utilization (0.0-1.0)
    #[must_use]
    pub const fn new(
        max_memory_bytes: u64,
        worker_threads: usize,
        memory_target: f64,
        cpu_target: f64,
    ) -> Self {
        Self {
            max_memory_bytes,
            worker_threads,
            memory_target,
            cpu_target,
        }
    }

    /// Create a resource budget preset for local development.
    ///
    /// Uses conservative settings suitable for development machines:
    /// - 512MB memory budget
    /// - 2 worker threads
    /// - 0.5 memory target
    /// - 0.5 CPU target
    #[must_use]
    pub const fn local() -> Self {
        Self {
            max_memory_bytes: 512 * 1024 * 1024, // 512MB
            worker_threads: 2,
            memory_target: 0.5,
            cpu_target: 0.5,
        }
    }

    /// Create a resource budget preset for production environments.
    ///
    /// Uses higher capacity settings suitable for production:
    /// - 4GB memory budget
    /// - 8 worker threads
    /// - 0.8 memory target
    /// - 0.7 CPU target
    #[must_use]
    pub const fn production() -> Self {
        Self {
            max_memory_bytes: 4 * 1024 * 1024 * 1024, // 4GB
            worker_threads: 8,
            memory_target: 0.8,
            cpu_target: 0.7,
        }
    }

    /// Estimate the maximum number of subscriptions based on the memory budget.
    ///
    /// Uses an estimate of ~10KB per subscription to calculate the maximum
    /// number of subscriptions that can fit within the target memory utilization.
    ///
    /// # Returns
    ///
    /// The estimated maximum number of subscriptions.
    #[must_use]
    pub fn estimate_max_subscriptions(&self) -> usize {
        let target_bytes = (self.max_memory_bytes as f64 * self.memory_target) as u64;
        (target_bytes / BYTES_PER_SUBSCRIPTION) as usize
    }
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self::local()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- ResourceBudget::new tests ---

    #[test]
    fn resource_budget_new_stores_values() {
        let budget = ResourceBudget::new(1024 * 1024, 4, 0.75, 0.65);

        assert_eq!(budget.max_memory_bytes, 1024 * 1024);
        assert_eq!(budget.worker_threads, 4);
        assert!((budget.memory_target - 0.75).abs() < f64::EPSILON);
        assert!((budget.cpu_target - 0.65).abs() < f64::EPSILON);
    }

    // --- ResourceBudget::local tests ---

    #[test]
    fn resource_budget_local_preset() {
        let budget = ResourceBudget::local();

        assert_eq!(budget.max_memory_bytes, 512 * 1024 * 1024); // 512MB
        assert_eq!(budget.worker_threads, 2);
        assert!((budget.memory_target - 0.5).abs() < f64::EPSILON);
        assert!((budget.cpu_target - 0.5).abs() < f64::EPSILON);
    }

    // --- ResourceBudget::production tests ---

    #[test]
    fn resource_budget_production_preset() {
        let budget = ResourceBudget::production();

        assert_eq!(budget.max_memory_bytes, 4 * 1024 * 1024 * 1024); // 4GB
        assert_eq!(budget.worker_threads, 8);
        assert!((budget.memory_target - 0.8).abs() < f64::EPSILON);
        assert!((budget.cpu_target - 0.7).abs() < f64::EPSILON);
    }

    // --- ResourceBudget::default tests ---

    #[test]
    fn resource_budget_default_is_local() {
        let default = ResourceBudget::default();
        let local = ResourceBudget::local();

        assert_eq!(default, local);
    }

    // --- ResourceBudget::estimate_max_subscriptions tests ---

    #[test]
    fn resource_budget_estimate_max_subscriptions_local() {
        let budget = ResourceBudget::local();
        let max_subs = budget.estimate_max_subscriptions();

        // 512MB * 0.5 = 256MB target, ~10KB per sub = ~26,214 subs
        // (256 * 1024 * 1024) / (10 * 1024) = 26,214
        assert_eq!(max_subs, 26_214);
    }

    #[test]
    fn resource_budget_estimate_max_subscriptions_production() {
        let budget = ResourceBudget::production();
        let max_subs = budget.estimate_max_subscriptions();

        // 4GB * 0.8 = 3.2GB target, ~10KB per sub = ~335,544 subs
        // (4 * 1024 * 1024 * 1024 * 0.8) / (10 * 1024) = 335,544
        assert_eq!(max_subs, 335_544);
    }

    #[test]
    fn resource_budget_estimate_max_subscriptions_custom() {
        // 100MB budget, 0.5 target = 50MB effective
        // 50MB / 10KB = 5,120 subscriptions
        let budget = ResourceBudget::new(100 * 1024 * 1024, 4, 0.5, 0.5);
        let max_subs = budget.estimate_max_subscriptions();

        assert_eq!(max_subs, 5_120);
    }

    #[test]
    fn resource_budget_estimate_max_subscriptions_zero_target() {
        let budget = ResourceBudget::new(1024 * 1024 * 1024, 4, 0.0, 0.5);
        let max_subs = budget.estimate_max_subscriptions();

        assert_eq!(max_subs, 0);
    }
}
