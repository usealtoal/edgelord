//! Telegram notification and command handling.
//!
//! Provides the [`TelegramNotifier`] for sending trade notifications and
//! handling bot commands. Spawns background workers for both outbound
//! messages and inbound command processing.
//!
//! Requires the `telegram` feature to be enabled.

use std::sync::Arc;

use teloxide::prelude::*;
use teloxide::types::{BotCommand, ParseMode};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::port::{inbound::runtime::RuntimeState, outbound::stats::StatsRecorder};
use crate::port::{outbound::notifier::Event, outbound::notifier::Notifier};

use super::auth::command_response_for_message;
use super::command::bot_commands;
use super::control::{RuntimeStats, TelegramControl};
use super::format::format_event_message;

/// Configuration for the Telegram notifier.
///
/// Controls which events trigger notifications and display limits for
/// bot command responses.
#[derive(Debug, Clone)]
pub struct TelegramConfig {
    /// Bot API token obtained from BotFather.
    pub bot_token: String,
    /// Target chat ID for notifications.
    pub chat_id: i64,
    /// Send notifications for detected opportunities (can be noisy).
    pub notify_opportunities: bool,
    /// Send notifications for executed trades.
    pub notify_executions: bool,
    /// Send notifications for risk-rejected opportunities.
    pub notify_risk_rejections: bool,
    /// Maximum positions to display in the /positions command response.
    pub position_display_limit: usize,
}

impl TelegramConfig {
    /// Create configuration from environment variables.
    ///
    /// Reads `TELEGRAM_BOT_TOKEN`, `TELEGRAM_CHAT_ID`, and optionally
    /// `TELEGRAM_NOTIFY_OPPORTUNITIES`. Returns `None` if required
    /// variables are missing or invalid.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let bot_token = std::env::var("TELEGRAM_BOT_TOKEN").ok()?;
        let chat_id = std::env::var("TELEGRAM_CHAT_ID")
            .ok()
            .and_then(|s| s.parse().ok())?;

        Some(Self {
            bot_token,
            chat_id,
            notify_opportunities: std::env::var("TELEGRAM_NOTIFY_OPPORTUNITIES")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            notify_executions: true,
            notify_risk_rejections: true,
            position_display_limit: 10,
        })
    }
}

/// Telegram notifier that sends messages to a chat.
///
/// Implements the [`Notifier`] trait and spawns background workers for
/// message delivery and command handling.
pub struct TelegramNotifier {
    /// Channel sender for queuing outbound notifications.
    sender: mpsc::UnboundedSender<Event>,
}

impl TelegramNotifier {
    /// Create a new Telegram notifier and spawn the background worker.
    ///
    /// This constructor creates a notification-only notifier without
    /// command handling capabilities.
    #[must_use]
    pub fn new(config: TelegramConfig) -> Self {
        Self::new_inner(config, None, None, None)
    }

    /// Create a notifier with command handling tied to application state.
    ///
    /// Enables basic bot commands like /status and /pause.
    #[must_use]
    pub fn new_with_control(config: TelegramConfig, state: Arc<dyn RuntimeState>) -> Self {
        Self::new_inner(config, Some(state), None, None)
    }

    /// Create a notifier with full command handling capabilities.
    ///
    /// Enables all bot commands including /stats and /positions.
    #[must_use]
    pub fn new_with_full_control(
        config: TelegramConfig,
        state: Arc<dyn RuntimeState>,
        stats_recorder: Arc<dyn StatsRecorder>,
        runtime_stats: Arc<RuntimeStats>,
    ) -> Self {
        Self::new_inner(
            config,
            Some(state),
            Some(stats_recorder),
            Some(runtime_stats),
        )
    }

    fn new_inner(
        config: TelegramConfig,
        state: Option<Arc<dyn RuntimeState>>,
        stats_recorder: Option<Arc<dyn StatsRecorder>>,
        runtime_stats: Option<Arc<RuntimeStats>>,
    ) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let worker_config = config.clone();

        // Spawn background task to handle outbound notifications.
        tokio::spawn(telegram_worker(worker_config, receiver));

        if let Some(state) = state {
            let control = if let (Some(recorder), Some(runtime)) = (stats_recorder, runtime_stats) {
                TelegramControl::with_config(
                    state,
                    recorder,
                    runtime,
                    config.position_display_limit,
                )
            } else {
                TelegramControl::new(state)
            };
            // Spawn background task to handle inbound bot commands.
            tokio::spawn(telegram_command_worker(config, control));
        }

        Self { sender }
    }
}

impl Notifier for TelegramNotifier {
    fn notify(&self, event: Event) {
        if self.sender.send(event).is_err() {
            warn!("Telegram notifier channel closed");
        }
    }
}

/// Background worker that sends Telegram messages.
async fn telegram_worker(config: TelegramConfig, mut receiver: mpsc::UnboundedReceiver<Event>) {
    let bot = Bot::new(&config.bot_token);
    let chat_id = ChatId(config.chat_id);

    info!(chat_id = config.chat_id, "Telegram notifier started");

    while let Some(event) = receiver.recv().await {
        let message = format_event_message(&event, &config);

        if let Some(text) = message {
            if let Err(e) = bot
                .send_message(chat_id, &text)
                .parse_mode(ParseMode::MarkdownV2)
                .await
            {
                error!(error = %e, "Failed to send Telegram message");
            }
        }
    }

    warn!("Telegram notifier worker shutting down");
}

/// Background worker that handles inbound Telegram commands.
async fn telegram_command_worker(config: TelegramConfig, control: TelegramControl) {
    let bot = Bot::new(&config.bot_token);
    let allowed_chat = ChatId(config.chat_id);

    // Register commands with Telegram so they appear in the "/" menu
    if let Err(e) = register_bot_commands(&bot).await {
        warn!(error = %e, "Failed to register bot commands with Telegram");
    }

    info!(
        chat_id = config.chat_id,
        "Telegram command listener started"
    );

    teloxide::repl(bot, move |bot: Bot, msg: Message| {
        let control = control.clone();
        async move {
            let Some(text) = msg.text() else {
                return respond(());
            };

            if let Some(response) =
                command_response_for_message(text, msg.chat.id, allowed_chat, &control)
            {
                if let Err(e) = bot.send_message(msg.chat.id, response).await {
                    error!(error = %e, "Failed to send Telegram command response");
                }
            }

            respond(())
        }
    })
    .await;
}

/// Register bot commands with Telegram for the "/" menu.
async fn register_bot_commands(bot: &Bot) -> Result<(), teloxide::RequestError> {
    let commands: Vec<BotCommand> = bot_commands()
        .into_iter()
        .map(|(cmd, desc)| BotCommand::new(cmd, desc))
        .collect();

    bot.set_my_commands(commands).await?;
    info!("Registered bot commands with Telegram");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Mutex to serialize tests that modify environment variables.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_from_env_missing_token() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("TELEGRAM_BOT_TOKEN");
        std::env::remove_var("TELEGRAM_CHAT_ID");

        assert!(TelegramConfig::from_env().is_none());
    }

    #[test]
    fn test_from_env_missing_chat_id() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("TELEGRAM_BOT_TOKEN", "test-token");
        std::env::remove_var("TELEGRAM_CHAT_ID");

        let result = TelegramConfig::from_env();
        assert!(result.is_none());

        std::env::remove_var("TELEGRAM_BOT_TOKEN");
    }

    #[test]
    fn test_from_env_invalid_chat_id() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("TELEGRAM_BOT_TOKEN", "test-token");
        std::env::set_var("TELEGRAM_CHAT_ID", "not-a-number");

        let result = TelegramConfig::from_env();
        assert!(result.is_none());

        std::env::remove_var("TELEGRAM_BOT_TOKEN");
        std::env::remove_var("TELEGRAM_CHAT_ID");
    }

    #[test]
    fn test_from_env_valid() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("TELEGRAM_BOT_TOKEN", "test-token");
        std::env::set_var("TELEGRAM_CHAT_ID", "12345");
        std::env::set_var("TELEGRAM_NOTIFY_OPPORTUNITIES", "true");

        let config = TelegramConfig::from_env().unwrap();
        assert_eq!(config.bot_token, "test-token");
        assert_eq!(config.chat_id, 12345);
        assert!(config.notify_opportunities);
        assert!(config.notify_executions);
        assert!(config.notify_risk_rejections);

        std::env::remove_var("TELEGRAM_BOT_TOKEN");
        std::env::remove_var("TELEGRAM_CHAT_ID");
        std::env::remove_var("TELEGRAM_NOTIFY_OPPORTUNITIES");
    }
}
