//! Profile and resource configuration for adaptive subscription management.

use serde::Deserialize;

use crate::domain::ResourceBudget;

/// Application profile for resource allocation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    /// Local development with conservative resource usage.
    #[default]
    Local,
    /// Production with higher resource capacity.
    Production,
    /// Custom profile using explicit ResourceConfig values.
    Custom,
}

/// Resource configuration for adaptive subscription management.
#[derive(Debug, Clone, Deserialize)]
pub struct ResourceConfig {
    /// Auto-detect system resources at startup.
    #[serde(default)]
    pub auto_detect: bool,
    /// Maximum memory budget in megabytes.
    #[serde(default)]
    pub max_memory_mb: Option<u64>,
    /// Number of worker threads.
    #[serde(default)]
    pub worker_threads: Option<usize>,
    /// Target memory utilization (0.0-1.0).
    #[serde(default = "default_memory_usage_target")]
    pub memory_usage_target: f64,
    /// Target CPU utilization (0.0-1.0).
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
    /// Convert to a ResourceBudget, using auto-detection if enabled.
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
