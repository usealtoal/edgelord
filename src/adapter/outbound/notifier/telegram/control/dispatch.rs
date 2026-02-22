use std::sync::Arc;

use chrono::Utc;

use crate::port::{inbound::runtime::RuntimeState, outbound::stats::StatsRecorder};

use super::super::command::{command_help, TelegramCommand};
use super::{RuntimeStats, TelegramControl, DEFAULT_POSITION_DISPLAY_LIMIT};

impl TelegramControl {
    /// Create a new control with just app state (minimal).
    #[must_use]
    pub fn new(state: Arc<dyn RuntimeState>) -> Self {
        Self {
            state,
            stats_recorder: None,
            runtime_stats: None,
            started_at: Utc::now(),
            position_display_limit: DEFAULT_POSITION_DISPLAY_LIMIT,
        }
    }

    /// Create a control with full dependencies and custom position display limit.
    #[must_use]
    pub fn with_config(
        state: Arc<dyn RuntimeState>,
        stats_recorder: Arc<dyn StatsRecorder>,
        runtime_stats: Arc<RuntimeStats>,
        position_display_limit: usize,
    ) -> Self {
        Self {
            state,
            stats_recorder: Some(stats_recorder),
            runtime_stats: Some(runtime_stats),
            started_at: Utc::now(),
            position_display_limit,
        }
    }

    /// Execute one parsed command and return response text.
    #[must_use]
    pub fn execute(&self, command: TelegramCommand) -> String {
        match command {
            TelegramCommand::Start | TelegramCommand::Help => command_help().to_string(),
            TelegramCommand::Status => self.status_text(),
            TelegramCommand::Health => self.health_text(),
            TelegramCommand::Positions => self.positions_text(),
            TelegramCommand::Stats => self.stats_text(),
            TelegramCommand::Pool => self.pool_text(),
            TelegramCommand::Markets => self.markets_text(),
            TelegramCommand::Version => self.version_text(),
            TelegramCommand::Pause => self.pause_text(),
            TelegramCommand::Resume => self.resume_text(),
            TelegramCommand::SetRisk { kind, value } => self.set_risk_text(kind, value),
        }
    }
}
