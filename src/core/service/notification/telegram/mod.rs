//! Telegram notification and command handling.
//!
//! Requires the `telegram` feature to be enabled.

mod command;
mod control;

use std::sync::Arc;

use teloxide::prelude::*;
use teloxide::types::ParseMode;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::app::AppState;

use self::command::{command_help, parse_command, CommandParseError};
use self::control::TelegramControl;
use super::{Event, Notifier};

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
        Self::new_inner(config, None)
    }

    /// Create a notifier and enable Telegram command handling tied to app state.
    #[must_use]
    pub fn new_with_control(config: TelegramConfig, state: Arc<AppState>) -> Self {
        Self::new_inner(config, Some(state))
    }

    fn new_inner(config: TelegramConfig, state: Option<Arc<AppState>>) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let worker_config = config.clone();

        // Spawn background task to handle outbound notifications.
        tokio::spawn(telegram_worker(worker_config, receiver));

        if let Some(state) = state {
            // Spawn background task to handle inbound bot commands.
            tokio::spawn(telegram_command_worker(config, TelegramControl::new(state)));
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
        Event::OpportunityDetected(e) if config.notify_opportunities => Some(format!(
            "🎯 *Opportunity Detected*\n\n\
                 Market: `{}`\n\
                 Question: {}\n\
                 Edge: {:.2}%\n\
                 Volume: ${:.2}\n\
                 Expected Profit: ${:.2}",
            e.market_id,
            escape_markdown(&e.question),
            e.edge * rust_decimal::Decimal::from(100),
            e.volume,
            e.expected_profit
        )),
        Event::ExecutionCompleted(e) if config.notify_executions => {
            let emoji = if e.success { "✅" } else { "❌" };
            Some(format!(
                "{} *Execution {}*\n\n\
                 Market: `{}`\n\
                 Details: {}",
                emoji,
                if e.success { "Success" } else { "Failed" },
                e.market_id,
                escape_markdown(&e.details)
            ))
        }
        Event::RiskRejected(e) if config.notify_risk_rejections => Some(format!(
            "⚠️ *Risk Rejected*\n\n\
                 Market: `{}`\n\
                 Reason: {}",
            e.market_id,
            escape_markdown(&e.reason)
        )),
        Event::CircuitBreakerActivated { reason } => Some(format!(
            "🚨 *CIRCUIT BREAKER ACTIVATED*\n\n\
                 Reason: {}\n\n\
                 All trading has been halted.",
            escape_markdown(reason)
        )),
        Event::CircuitBreakerReset => {
            Some("✅ *Circuit Breaker Reset*\n\nTrading has resumed.".to_string())
        }
        Event::DailySummary(e) => Some(format!(
            "📊 *Daily Summary - {}*\n\n\
                 Opportunities: {}\n\
                 Trades Executed: {}\n\
                 Successful: {}\n\
                 Total Profit: ${:.2}\n\
                 Current Exposure: ${:.2}",
            e.date,
            e.opportunities_detected,
            e.trades_executed,
            e.trades_successful,
            e.total_profit,
            e.current_exposure
        )),
        _ => None,
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
    fn test_command_response_for_authorized_command() {
        let state = Arc::new(AppState::default());
        let control = TelegramControl::new(state);
        let chat = ChatId(42);

        let response = command_response_for_message("/status", chat, chat, &control).unwrap();
        assert!(response.contains("Edgelord status"));
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
        assert!(response.contains("Edgelord Telegram commands"));
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
