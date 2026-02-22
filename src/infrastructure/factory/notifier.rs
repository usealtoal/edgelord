//! Notifier registry factory.

use std::sync::Arc;

use tracing::{info, warn};

use crate::application::state::AppState;
use crate::infrastructure::config::settings::Config;
use crate::port::outbound::notifier::{LogNotifier, NotifierRegistry};
use crate::port::outbound::stats::StatsRecorder;

#[cfg(feature = "telegram")]
use crate::adapter::outbound::notifier::telegram::control::RuntimeStats;
#[cfg(feature = "telegram")]
use crate::adapter::outbound::notifier::telegram::notifier::{TelegramConfig, TelegramNotifier};

/// Build notifier registry from configuration.
///
/// When the `telegram` feature is enabled, this also creates a `RuntimeStats`
/// instance that should be updated by the orchestrator with pool and market info.
#[cfg(feature = "telegram")]
pub fn build_notifier_registry(
    config: &Config,
    state: Arc<AppState>,
    stats_recorder: Arc<dyn StatsRecorder>,
) -> (NotifierRegistry, Option<Arc<RuntimeStats>>) {
    let mut registry = NotifierRegistry::new();
    registry.register(Box::new(LogNotifier));

    let runtime_stats = if config.telegram.enabled {
        if let Some(tg_config) = TelegramConfig::from_env() {
            let tg_config = TelegramConfig {
                notify_opportunities: config.telegram.notify_opportunities,
                notify_executions: config.telegram.notify_executions,
                notify_risk_rejections: config.telegram.notify_risk_rejections,
                position_display_limit: config.telegram.position_display_limit,
                ..tg_config
            };
            let runtime_stats = Arc::new(RuntimeStats::new());
            let runtime: Arc<dyn crate::port::inbound::runtime::RuntimeState> = state.clone();
            registry.register(Box::new(TelegramNotifier::new_with_full_control(
                tg_config,
                runtime,
                stats_recorder,
                Arc::clone(&runtime_stats),
            )));
            info!("Telegram notifier enabled with full control");
            Some(runtime_stats)
        } else {
            warn!("Telegram enabled but TELEGRAM_BOT_TOKEN or TELEGRAM_CHAT_ID not set");
            None
        }
    } else {
        None
    };

    (registry, runtime_stats)
}

/// Build notifier registry from configuration (non-telegram variant).
#[cfg(not(feature = "telegram"))]
pub fn build_notifier_registry(
    _config: &Config,
    _state: Arc<AppState>,
    _stats_recorder: Arc<dyn StatsRecorder>,
) -> (NotifierRegistry, Option<()>) {
    let mut registry = NotifierRegistry::new();
    registry.register(Box::new(LogNotifier));
    (registry, None)
}
