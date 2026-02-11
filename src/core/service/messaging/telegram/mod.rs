//! Telegram notification and command handling.
//!
//! Requires the `telegram` feature to be enabled.

mod command;
mod control;

use std::sync::Arc;

use teloxide::prelude::*;
use teloxide::types::{BotCommand, ParseMode};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::app::AppState;
use crate::core::service::StatsRecorder;

use self::command::{bot_commands, command_help, parse_command, CommandParseError};
use self::control::TelegramControl;
use super::{Event, Notifier};

pub use self::control::RuntimeStats;

/// Configuration for Telegram notifier.
#[derive(Debug, Clone)]
pub struct TelegramConfig {
    /// Bot token from @`BotFather`.
    pub bot_token: String,
    /// Chat ID to send notifications to.
    pub chat_id: i64,
    /// Whether to send opportunity alerts (can be noisy).
    pub notify_opportunities: bool,
    /// Whether to send execution alerts.
    pub notify_executions: bool,
    /// Whether to send risk rejections.
    pub notify_risk_rejections: bool,
    /// Maximum positions to display in /positions command.
    pub position_display_limit: usize,
}

impl TelegramConfig {
    /// Create config from environment variables.
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
pub struct TelegramNotifier {
    sender: mpsc::UnboundedSender<Event>,
}

impl TelegramNotifier {
    /// Create a new Telegram notifier and spawn the background task.
    #[must_use]
    pub fn new(config: TelegramConfig) -> Self {
        Self::new_inner(config, None, None, None)
    }

    /// Create a notifier and enable Telegram command handling tied to app state.
    #[must_use]
    pub fn new_with_control(config: TelegramConfig, state: Arc<AppState>) -> Self {
        Self::new_inner(config, Some(state), None, None)
    }

    /// Create a notifier with full dependencies for all commands.
    #[must_use]
    pub fn new_with_full_control(
        config: TelegramConfig,
        state: Arc<AppState>,
        stats_recorder: Arc<StatsRecorder>,
        runtime_stats: Arc<RuntimeStats>,
    ) -> Self {
        Self::new_inner(config, Some(state), Some(stats_recorder), Some(runtime_stats))
    }

    fn new_inner(
        config: TelegramConfig,
        state: Option<Arc<AppState>>,
        stats_recorder: Option<Arc<StatsRecorder>>,
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

fn command_response_for_message(
    text: &str,
    incoming_chat: ChatId,
    allowed_chat: ChatId,
    control: &TelegramControl,
) -> Option<String> {
    if incoming_chat != allowed_chat {
        warn!(
            chat_id = incoming_chat.0,
            "Ignoring Telegram message from unauthorized chat"
        );
        return None;
    }

    match parse_command(text) {
        Ok(command) => Some(control.execute(command)),
        Err(CommandParseError::NotACommand) => None,
        Err(err) => Some(format!("Invalid command: {err}\n\n{}", command_help())),
    }
}

fn format_event_message(event: &Event, config: &TelegramConfig) -> Option<String> {
    match event {
        Event::OpportunityDetected(e) if config.notify_opportunities => {
            let question = truncate(&e.question, 60);

            Some(format!(
                "*Opportunity Detected*\n\
                \n\
                {}\n\
                Edge: {:.2}%\n\
                Volume: ${:.2}\n\
                Expected: \\+${:.2}",
                escape_markdown(&question),
                e.edge * rust_decimal::Decimal::from(100),
                e.volume,
                e.expected_profit
            ))
        }
        Event::ExecutionCompleted(e) if config.notify_executions => {
            let title = if e.success {
                "Trade Executed"
            } else {
                "Execution Failed"
            };

            let market_display = truncate(&e.market_id, 16);

            Some(format!(
                "*{}*\n\
                \n\
                Market: {}\n\
                Details: {}",
                title,
                escape_markdown(&market_display),
                escape_markdown(&e.details)
            ))
        }
        Event::RiskRejected(e) if config.notify_risk_rejections => {
            let market_display = truncate(&e.market_id, 16);

            Some(format!(
                "*Risk Check Failed*\n\
                \n\
                Market: {}\n\
                Reason: {}",
                escape_markdown(&market_display),
                escape_markdown(&e.reason)
            ))
        }
        Event::CircuitBreakerActivated { reason } => Some(format!(
            "*Circuit Breaker Activated*\n\
            \n\
            Reason: {}\n\
            Trading halted",
            escape_markdown(reason)
        )),
        Event::CircuitBreakerReset => Some(
            "*Circuit Breaker Reset*\n\
            \n\
            Trading resumed"
                .to_string(),
        ),
        Event::DailySummary(e) => Some(format!(
            "*Daily Summary — {}*\n\
            \n\
            Opportunities: {}\n\
            Trades: {}\n\
            Successful: {}\n\
            Profit: \\+${:.2}\n\
            Exposure: ${:.2}",
            escape_markdown(&e.date.to_string()),
            e.opportunities_detected,
            e.trades_executed,
            e.trades_successful,
            e.total_profit,
            e.current_exposure
        )),
        _ => None,
    }
}

/// Truncate a string with ellipsis (Unicode-safe).
fn truncate(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count > max_chars {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}

/// Escape special characters for Telegram `MarkdownV2`.
fn escape_markdown(text: &str) -> String {
    let special_chars = [
        '_', '*', '[', ']', '(', ')', '~', '`', '>', '#', '+', '-', '=', '|', '{', '}', '.', '!',
    ];
    let mut result = String::with_capacity(text.len() * 2);

    for c in text.chars() {
        if special_chars.contains(&c) {
            result.push('\\');
        }
        result.push(c);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    use crate::app::AppState;

    /// Mutex to serialize tests that modify environment variables.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_escape_markdown() {
        assert_eq!(escape_markdown("hello"), "hello");
        assert_eq!(escape_markdown("hello_world"), "hello\\_world");
        assert_eq!(escape_markdown("*bold*"), "\\*bold\\*");
        assert_eq!(escape_markdown("test.com"), "test\\.com");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 5), "hello...");
        assert_eq!(truncate("ab", 2), "ab");
    }

    #[test]
    fn test_truncate_unicode() {
        // Handles multi-byte UTF-8 characters without panic
        assert_eq!(truncate("日本語テスト", 3), "日本語...");
        assert_eq!(truncate("café", 4), "café");
        assert_eq!(truncate("🎯🚀💰", 2), "🎯🚀...");
    }

    #[test]
    fn test_command_response_for_authorized_command() {
        let state = Arc::new(AppState::default());
        let control = TelegramControl::new(state);
        let chat = ChatId(42);

        let response = command_response_for_message("/status", chat, chat, &control).unwrap();
        assert!(response.contains("Edgelord Status"));
    }

    #[test]
    fn test_command_response_ignores_unauthorized_chat() {
        let state = Arc::new(AppState::default());
        let control = TelegramControl::new(state);

        let response = command_response_for_message("/status", ChatId(7), ChatId(42), &control);
        assert!(response.is_none());
    }

    #[test]
    fn test_command_response_for_invalid_command() {
        let state = Arc::new(AppState::default());
        let control = TelegramControl::new(state);
        let chat = ChatId(42);

        let response = command_response_for_message("/bad", chat, chat, &control).unwrap();
        assert!(response.contains("Invalid command"));
        assert!(response.contains("Edgelord Commands"));
    }

    #[test]
    fn test_command_response_ignores_non_command_text() {
        let state = Arc::new(AppState::default());
        let control = TelegramControl::new(state);
        let chat = ChatId(42);

        let response = command_response_for_message("hello", chat, chat, &control);
        assert!(response.is_none());
    }

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
