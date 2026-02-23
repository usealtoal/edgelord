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
    use crate::port::outbound::notifier::{ExecutionEvent, OpportunityEvent, RiskEvent};
    use rust_decimal_macros::dec;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    /// Mutex to serialize tests that modify environment variables.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    // -------------------------------------------------------------------------
    // TelegramConfig::from_env tests
    // -------------------------------------------------------------------------

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

    #[test]
    fn test_from_env_notify_opportunities_false() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("TELEGRAM_BOT_TOKEN", "test-token");
        std::env::set_var("TELEGRAM_CHAT_ID", "12345");
        std::env::set_var("TELEGRAM_NOTIFY_OPPORTUNITIES", "false");

        let config = TelegramConfig::from_env().unwrap();
        assert!(!config.notify_opportunities);

        std::env::remove_var("TELEGRAM_BOT_TOKEN");
        std::env::remove_var("TELEGRAM_CHAT_ID");
        std::env::remove_var("TELEGRAM_NOTIFY_OPPORTUNITIES");
    }

    #[test]
    fn test_from_env_notify_opportunities_one() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("TELEGRAM_BOT_TOKEN", "test-token");
        std::env::set_var("TELEGRAM_CHAT_ID", "12345");
        std::env::set_var("TELEGRAM_NOTIFY_OPPORTUNITIES", "1");

        let config = TelegramConfig::from_env().unwrap();
        assert!(config.notify_opportunities);

        std::env::remove_var("TELEGRAM_BOT_TOKEN");
        std::env::remove_var("TELEGRAM_CHAT_ID");
        std::env::remove_var("TELEGRAM_NOTIFY_OPPORTUNITIES");
    }

    #[test]
    fn test_from_env_notify_opportunities_unset() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("TELEGRAM_BOT_TOKEN", "test-token");
        std::env::set_var("TELEGRAM_CHAT_ID", "12345");
        std::env::remove_var("TELEGRAM_NOTIFY_OPPORTUNITIES");

        let config = TelegramConfig::from_env().unwrap();
        // Default is false when unset
        assert!(!config.notify_opportunities);

        std::env::remove_var("TELEGRAM_BOT_TOKEN");
        std::env::remove_var("TELEGRAM_CHAT_ID");
    }

    #[test]
    fn test_from_env_negative_chat_id() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("TELEGRAM_BOT_TOKEN", "test-token");
        // Telegram group chat IDs can be negative
        std::env::set_var("TELEGRAM_CHAT_ID", "-123456789");

        let config = TelegramConfig::from_env().unwrap();
        assert_eq!(config.chat_id, -123456789);

        std::env::remove_var("TELEGRAM_BOT_TOKEN");
        std::env::remove_var("TELEGRAM_CHAT_ID");
    }

    #[test]
    fn test_from_env_empty_token() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("TELEGRAM_BOT_TOKEN", "");
        std::env::set_var("TELEGRAM_CHAT_ID", "12345");

        // Empty token is technically valid (will fail at API level)
        let config = TelegramConfig::from_env().unwrap();
        assert_eq!(config.bot_token, "");

        std::env::remove_var("TELEGRAM_BOT_TOKEN");
        std::env::remove_var("TELEGRAM_CHAT_ID");
    }

    #[test]
    fn test_from_env_chat_id_overflow() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("TELEGRAM_BOT_TOKEN", "test-token");
        // i64::MAX + 1 would overflow
        std::env::set_var("TELEGRAM_CHAT_ID", "99999999999999999999");

        let result = TelegramConfig::from_env();
        assert!(result.is_none());

        std::env::remove_var("TELEGRAM_BOT_TOKEN");
        std::env::remove_var("TELEGRAM_CHAT_ID");
    }

    // -------------------------------------------------------------------------
    // TelegramConfig struct tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_config_clone() {
        let config = TelegramConfig {
            bot_token: "token".to_string(),
            chat_id: 12345,
            notify_opportunities: true,
            notify_executions: true,
            notify_risk_rejections: false,
            position_display_limit: 5,
        };

        let cloned = config.clone();
        assert_eq!(cloned.bot_token, "token");
        assert_eq!(cloned.chat_id, 12345);
        assert!(cloned.notify_opportunities);
        assert!(cloned.notify_executions);
        assert!(!cloned.notify_risk_rejections);
        assert_eq!(cloned.position_display_limit, 5);
    }

    #[test]
    fn test_config_debug() {
        let config = TelegramConfig {
            bot_token: "secret-token".to_string(),
            chat_id: 12345,
            notify_opportunities: true,
            notify_executions: true,
            notify_risk_rejections: true,
            position_display_limit: 10,
        };

        let debug = format!("{:?}", config);
        assert!(debug.contains("TelegramConfig"));
        assert!(debug.contains("12345"));
    }

    // -------------------------------------------------------------------------
    // Notifier trait implementation tests (using mock channel)
    // -------------------------------------------------------------------------

    /// A mock notifier that counts events instead of sending to Telegram.
    struct MockNotifier {
        event_count: Arc<AtomicUsize>,
    }

    impl MockNotifier {
        fn new() -> (Self, Arc<AtomicUsize>) {
            let count = Arc::new(AtomicUsize::new(0));
            (
                Self {
                    event_count: Arc::clone(&count),
                },
                count,
            )
        }
    }

    impl Notifier for MockNotifier {
        fn notify(&self, _event: Event) {
            self.event_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn test_mock_notifier_counts_events() {
        let (notifier, count) = MockNotifier::new();

        assert_eq!(count.load(Ordering::SeqCst), 0);

        notifier.notify(Event::CircuitBreakerReset);
        assert_eq!(count.load(Ordering::SeqCst), 1);

        notifier.notify(Event::CircuitBreakerActivated {
            reason: "test".to_string(),
        });
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    // -------------------------------------------------------------------------
    // Event creation helper tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_opportunity_event_creation() {
        let event = OpportunityEvent {
            market_id: "test-market".to_string(),
            question: "Will it happen?".to_string(),
            edge: dec!(0.05),
            volume: dec!(100),
            expected_profit: dec!(5),
        };

        assert_eq!(event.market_id, "test-market");
        assert_eq!(event.question, "Will it happen?");
        assert_eq!(event.edge, dec!(0.05));
        assert_eq!(event.volume, dec!(100));
        assert_eq!(event.expected_profit, dec!(5));
    }

    #[test]
    fn test_execution_event_success() {
        let event = ExecutionEvent {
            market_id: "market-1".to_string(),
            success: true,
            details: "Order filled".to_string(),
        };

        assert!(event.success);
        assert_eq!(event.details, "Order filled");
    }

    #[test]
    fn test_execution_event_failure() {
        let event = ExecutionEvent {
            market_id: "market-1".to_string(),
            success: false,
            details: "Insufficient balance".to_string(),
        };

        assert!(!event.success);
        assert_eq!(event.details, "Insufficient balance");
    }

    #[test]
    fn test_risk_event_creation() {
        let event = RiskEvent {
            market_id: "market-2".to_string(),
            reason: "Exceeds position limit".to_string(),
        };

        assert_eq!(event.market_id, "market-2");
        assert_eq!(event.reason, "Exceeds position limit");
    }

    // -------------------------------------------------------------------------
    // Event enum tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_event_clone() {
        let event = Event::CircuitBreakerActivated {
            reason: "test reason".to_string(),
        };
        let cloned = event.clone();

        if let Event::CircuitBreakerActivated { reason } = cloned {
            assert_eq!(reason, "test reason");
        } else {
            panic!("Expected CircuitBreakerActivated");
        }
    }

    #[test]
    fn test_event_debug() {
        let event = Event::CircuitBreakerReset;
        let debug = format!("{:?}", event);
        assert!(debug.contains("CircuitBreakerReset"));
    }

    // -------------------------------------------------------------------------
    // Config defaults
    // -------------------------------------------------------------------------

    #[test]
    fn test_config_defaults_from_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("TELEGRAM_BOT_TOKEN", "token");
        std::env::set_var("TELEGRAM_CHAT_ID", "1");
        std::env::remove_var("TELEGRAM_NOTIFY_OPPORTUNITIES");

        let config = TelegramConfig::from_env().unwrap();

        // Check default values
        assert!(!config.notify_opportunities); // Default false
        assert!(config.notify_executions); // Default true
        assert!(config.notify_risk_rejections); // Default true
        assert_eq!(config.position_display_limit, 10); // Default 10

        std::env::remove_var("TELEGRAM_BOT_TOKEN");
        std::env::remove_var("TELEGRAM_CHAT_ID");
    }
}

/// Integration tests that require real Telegram API access.
///
/// These tests are only run when the `telegram-integration` feature is enabled.
/// They require the following environment variables to be set:
/// - `TELEGRAM_BOT_TOKEN`: A valid Telegram bot token from BotFather
/// - `TELEGRAM_CHAT_ID`: A valid chat ID where the bot has permission to send messages
///
/// Run with: `cargo test --lib --features telegram-integration telegram_integration`
#[cfg(all(test, feature = "telegram-integration"))]
mod integration_tests {
    use super::*;
    use crate::port::outbound::notifier::{
        ExecutionEvent, OpportunityEvent, RelationDetail, RelationsEvent, RiskEvent, SummaryEvent,
    };
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn integration_config() -> Option<TelegramConfig> {
        // Only run if both env vars are set
        let token = std::env::var("TELEGRAM_BOT_TOKEN").ok()?;
        let chat_id = std::env::var("TELEGRAM_CHAT_ID")
            .ok()
            .and_then(|s| s.parse().ok())?;

        if token.is_empty() {
            return None;
        }

        Some(TelegramConfig {
            bot_token: token,
            chat_id,
            notify_opportunities: true,
            notify_executions: true,
            notify_risk_rejections: true,
            position_display_limit: 10,
        })
    }

    /// Helper to skip test if integration config is not available.
    macro_rules! require_integration_config {
        () => {
            match integration_config() {
                Some(config) => config,
                None => {
                    eprintln!(
                        "Skipping integration test: TELEGRAM_BOT_TOKEN and TELEGRAM_CHAT_ID must be set"
                    );
                    return;
                }
            }
        };
    }

    #[tokio::test]
    async fn test_send_opportunity_notification() {
        let config = require_integration_config!();

        let notifier = TelegramNotifier::new(config);

        let event = Event::OpportunityDetected(OpportunityEvent {
            market_id: "integration-test-market".to_string(),
            question: "[Integration Test] Will this test pass?".to_string(),
            edge: dec!(0.05),
            volume: dec!(100),
            expected_profit: dec!(5.00),
        });

        notifier.notify(event);

        // Give the async worker time to send the message
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }

    #[tokio::test]
    async fn test_send_execution_notification() {
        let config = require_integration_config!();

        let notifier = TelegramNotifier::new(config);

        let event = Event::ExecutionCompleted(ExecutionEvent {
            market_id: "integration-test-market".to_string(),
            success: true,
            details: "[Integration Test] Order 12345 filled successfully".to_string(),
        });

        notifier.notify(event);

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }

    #[tokio::test]
    async fn test_send_risk_rejection_notification() {
        let config = require_integration_config!();

        let notifier = TelegramNotifier::new(config);

        let event = Event::RiskRejected(RiskEvent {
            market_id: "integration-test-market".to_string(),
            reason: "[Integration Test] Position would exceed max exposure limit".to_string(),
        });

        notifier.notify(event);

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }

    #[tokio::test]
    async fn test_send_circuit_breaker_notifications() {
        let config = require_integration_config!();

        let notifier = TelegramNotifier::new(config);

        // Activate
        notifier.notify(Event::CircuitBreakerActivated {
            reason: "[Integration Test] Simulated circuit breaker activation".to_string(),
        });

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // Reset
        notifier.notify(Event::CircuitBreakerReset);

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }

    #[tokio::test]
    async fn test_send_daily_summary_notification() {
        let config = require_integration_config!();

        let notifier = TelegramNotifier::new(config);

        let event = Event::DailySummary(SummaryEvent {
            date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            opportunities_detected: 100,
            trades_executed: 25,
            trades_successful: 20,
            total_profit: dec!(150.50),
            current_exposure: dec!(500),
        });

        notifier.notify(event);

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }

    #[tokio::test]
    async fn test_send_relations_discovered_notification() {
        let config = require_integration_config!();

        let notifier = TelegramNotifier::new(config);

        let event = Event::RelationsDiscovered(RelationsEvent {
            relations_count: 2,
            relations: vec![
                RelationDetail {
                    relation_type: "mutually_exclusive".to_string(),
                    confidence: 0.95,
                    market_questions: vec![
                        "[Integration Test] Will Team A win?".to_string(),
                        "[Integration Test] Will Team B win?".to_string(),
                    ],
                    reasoning: "Only one team can win the championship".to_string(),
                },
                RelationDetail {
                    relation_type: "implies".to_string(),
                    confidence: 0.85,
                    market_questions: vec![
                        "[Integration Test] Will X happen?".to_string(),
                        "[Integration Test] Will Y happen?".to_string(),
                    ],
                    reasoning: "If X happens, Y must also happen".to_string(),
                },
            ],
        });

        notifier.notify(event);

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }

    #[tokio::test]
    async fn test_send_multiple_notifications_rapidly() {
        let config = require_integration_config!();

        let notifier = TelegramNotifier::new(config);

        // Send multiple notifications rapidly to test queuing
        for i in 0..5 {
            notifier.notify(Event::OpportunityDetected(OpportunityEvent {
                market_id: format!("rapid-test-{}", i),
                question: format!("[Integration Test] Rapid notification #{}", i),
                edge: dec!(0.01),
                volume: dec!(10),
                expected_profit: dec!(0.1),
            }));
        }

        // Give enough time for all messages to be sent
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }

    #[tokio::test]
    async fn test_notification_with_special_characters() {
        let config = require_integration_config!();

        let notifier = TelegramNotifier::new(config);

        // Test with characters that need escaping in MarkdownV2
        let event = Event::ExecutionCompleted(ExecutionEvent {
            market_id: "test_market-123.456".to_string(),
            success: true,
            details: "[Integration Test] Special chars: *bold* _italic_ `code` [link](url)"
                .to_string(),
        });

        notifier.notify(event);

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
}
