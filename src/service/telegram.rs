//! Telegram notification implementation.
//!
//! Requires the `telegram` feature to be enabled.

use teloxide::prelude::*;
use teloxide::types::ParseMode;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use super::{Event, Notifier};

/// Configuration for Telegram notifier.
#[derive(Debug, Clone)]
pub struct TelegramConfig {
    /// Bot token from @BotFather.
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
    pub fn new(config: TelegramConfig) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();

        // Spawn background task to handle messages
        tokio::spawn(telegram_worker(config, receiver));

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
        let message = match &event {
            Event::OpportunityDetected(e) if config.notify_opportunities => {
                Some(format!(
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
                ))
            }
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
            Event::RiskRejected(e) if config.notify_risk_rejections => {
                Some(format!(
                    "⚠️ *Risk Rejected*\n\n\
                     Market: `{}`\n\
                     Reason: {}",
                    e.market_id,
                    escape_markdown(&e.reason)
                ))
            }
            Event::CircuitBreakerActivated { reason } => {
                Some(format!(
                    "🚨 *CIRCUIT BREAKER ACTIVATED*\n\n\
                     Reason: {}\n\n\
                     All trading has been halted.",
                    escape_markdown(reason)
                ))
            }
            Event::CircuitBreakerReset => {
                Some("✅ *Circuit Breaker Reset*\n\nTrading has resumed.".to_string())
            }
            Event::DailySummary(e) => {
                Some(format!(
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
                ))
            }
            _ => None,
        };

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

    warn!("Telegram worker shutting down");
}

/// Escape special characters for Telegram MarkdownV2.
fn escape_markdown(text: &str) -> String {
    let special_chars = ['_', '*', '[', ']', '(', ')', '~', '`', '>', '#', '+', '-', '=', '|', '{', '}', '.', '!'];
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
    use std::sync::Mutex;

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
    fn test_from_env_missing_token() {
        let _guard = ENV_LOCK.lock().unwrap();
        // Clear env vars
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

        // Cleanup
        std::env::remove_var("TELEGRAM_BOT_TOKEN");
    }

    #[test]
    fn test_from_env_invalid_chat_id() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("TELEGRAM_BOT_TOKEN", "test-token");
        std::env::set_var("TELEGRAM_CHAT_ID", "not-a-number");

        let result = TelegramConfig::from_env();
        assert!(result.is_none());

        // Cleanup
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

        // Cleanup
        std::env::remove_var("TELEGRAM_BOT_TOKEN");
        std::env::remove_var("TELEGRAM_CHAT_ID");
        std::env::remove_var("TELEGRAM_NOTIFY_OPPORTUNITIES");
    }

    #[test]
    fn test_escape_markdown_all_special_chars() {
        let special = "_*[]()~`>#+-=|{}.!";
        let escaped = escape_markdown(special);
        assert_eq!(escaped, "\\_\\*\\[\\]\\(\\)\\~\\`\\>\\#\\+\\-\\=\\|\\{\\}\\.\\!");
    }

    #[test]
    fn test_escape_markdown_empty() {
        assert_eq!(escape_markdown(""), "");
    }
}
