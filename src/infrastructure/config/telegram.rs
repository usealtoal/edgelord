//! Telegram notification configuration.
//!
//! Provides configuration for Telegram bot notifications. Requires
//! `TELEGRAM_BOT_TOKEN` and `TELEGRAM_CHAT_ID` environment variables.

use serde::Deserialize;

const fn default_true() -> bool {
    true
}

/// Telegram notification configuration.
///
/// Controls which events trigger Telegram notifications and display settings.
/// The bot token and chat ID are read from environment variables.
#[derive(Debug, Clone, Deserialize)]
pub struct TelegramAppConfig {
    /// Enable Telegram notifications.
    ///
    /// When false, no Telegram messages are sent regardless of other settings.
    /// Defaults to false.
    #[serde(default)]
    pub enabled: bool,

    /// Send alerts for detected opportunities.
    ///
    /// Can be noisy in active markets. Defaults to false.
    #[serde(default)]
    pub notify_opportunities: bool,

    /// Send alerts for trade executions.
    ///
    /// Notifies when trades are successfully executed. Defaults to true.
    #[serde(default = "default_true")]
    pub notify_executions: bool,

    /// Send alerts when trades are rejected by risk checks.
    ///
    /// Useful for monitoring risk limit hits. Defaults to true.
    #[serde(default = "default_true")]
    pub notify_risk_rejections: bool,

    /// Interval for polling runtime statistics in seconds.
    ///
    /// Controls how often the `/stats` command data is refreshed.
    /// Defaults to 30.
    #[serde(default = "default_stats_interval_secs")]
    pub stats_interval_secs: u64,

    /// Maximum number of positions to display in status messages.
    ///
    /// Limits output length for readability. Defaults to 10.
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
