//! Profile and resource configuration for adaptive subscription management.
//!
//! Provides configuration for resource budgets that control how many
//! subscriptions the system can handle based on available memory and CPU.

use serde::Deserialize;

use crate::infrastructure::governor::resource::ResourceBudget;

/// Application profile for resource allocation.
///
/// Provides preset resource configurations suitable for different deployment
/// scenarios.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    /// Local development with conservative resource usage.
    ///
    /// Uses lower memory and thread limits suitable for development machines.
    #[default]
    Local,
    /// Production with higher resource capacity.
    ///
    /// Uses higher limits suitable for dedicated servers.
    Production,
    /// Custom profile using explicit [`ResourceConfig`] values.
    ///
    /// Ignores preset limits and uses only explicitly configured values.
    Custom,
}

/// Resource configuration for adaptive subscription management.
///
/// Fine-grained control over memory and CPU budgets. Used by the governor
/// to determine maximum subscription capacity.
#[derive(Debug, Clone, Deserialize)]
pub struct ResourceConfig {
    /// Enable automatic system resource detection at startup.
    ///
    /// When true, detects available memory and CPU cores automatically.
    /// Defaults to false.
    #[serde(default)]
    pub auto_detect: bool,

    /// Maximum memory budget in megabytes.
    ///
    /// When set, overrides profile-based and auto-detected values.
    #[serde(default)]
    pub max_memory_mb: Option<u64>,

    /// Number of worker threads.
    ///
    /// When set, overrides profile-based and auto-detected values.
    #[serde(default)]
    pub worker_threads: Option<usize>,

    /// Target memory utilization as a fraction (0.0 to 1.0).
    ///
    /// The governor aims to keep memory usage at or below this fraction
    /// of the budget. Defaults to 0.80 (80%).
    #[serde(default = "default_memory_usage_target")]
    pub memory_usage_target: f64,

    /// Target CPU utilization as a fraction (0.0 to 1.0).
    ///
    /// The governor aims to keep CPU usage at or below this fraction
    /// of available capacity. Defaults to 0.70 (70%).
    #[serde(default = "default_cpu_usage_target")]
    pub cpu_usage_target: f64,
}

fn default_memory_usage_target() -> f64 {
    0.80
}

fn default_cpu_usage_target() -> f64 {
    0.70
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            auto_detect: false,
            max_memory_mb: None,
            worker_threads: None,
            memory_usage_target: default_memory_usage_target(),
            cpu_usage_target: default_cpu_usage_target(),
        }
    }
}

impl ResourceConfig {
    /// Convert to a [`ResourceBudget`] for use by the governor.
    ///
    /// Combines profile-based defaults with explicit overrides and
    /// auto-detected values (if enabled) to produce the final budget.
    #[must_use]
    pub fn to_budget(&self, profile: Profile) -> ResourceBudget {
        // Start with profile-based defaults
        let base = match profile {
            Profile::Local => ResourceBudget::local(),
            Profile::Production => ResourceBudget::production(),
            Profile::Custom => ResourceBudget::local(), // Start with local as base for custom
        };

        // Determine memory bytes
        let max_memory_bytes = if let Some(mb) = self.max_memory_mb {
            mb * 1024 * 1024
        } else if self.auto_detect {
            // Use system memory if auto-detect enabled (fallback to base)
            base.max_memory_bytes
        } else {
            base.max_memory_bytes
        };

        // Determine worker threads
        let worker_threads = if let Some(threads) = self.worker_threads {
            threads
        } else if self.auto_detect {
            num_cpus::get()
        } else {
            base.worker_threads
        };

        ResourceBudget::new(
            max_memory_bytes,
            worker_threads,
            self.memory_usage_target,
            self.cpu_usage_target,
        )
    }
}
