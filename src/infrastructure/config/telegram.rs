//! Telegram notification configuration.

use serde::Deserialize;

const fn default_true() -> bool {
    true
}

/// Telegram notification configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct TelegramAppConfig {
    /// Enable telegram notifications.
    #[serde(default)]
    pub enabled: bool,
    /// Send opportunity alerts (can be noisy).
    #[serde(default)]
    pub notify_opportunities: bool,
    /// Send execution alerts.
    #[serde(default = "default_true")]
    pub notify_executions: bool,
    /// Send risk rejection alerts.
    #[serde(default = "default_true")]
    pub notify_risk_rejections: bool,
    /// Stats polling interval in seconds (default: 30).
    #[serde(default = "default_stats_interval_secs")]
    pub stats_interval_secs: u64,
    /// Maximum positions to display (default: 10).
    #[serde(default = "default_position_display_limit")]
    pub position_display_limit: usize,
}

const fn default_stats_interval_secs() -> u64 {
    30
}

const fn default_position_display_limit() -> usize {
    10
}

impl Default for TelegramAppConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            notify_opportunities: false,
            notify_executions: default_true(),
            notify_risk_rejections: default_true(),
            stats_interval_secs: default_stats_interval_secs(),
            position_display_limit: default_position_display_limit(),
        }
    }
}
