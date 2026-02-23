//! Authorization for Telegram command handling.

use teloxide::types::ChatId;
use tracing::warn;

use super::command::{command_help, parse_command, CommandParseError};
use super::control::TelegramControl;

/// Process a message and return a response if it's an authorized command.
///
/// Returns `None` for:
/// - Messages from unauthorized chats
/// - Messages that are not commands (don't start with `/`)
///
/// Returns `Some(response)` for:
/// - Valid commands from the authorized chat
/// - Invalid commands (with error message and help)
pub fn command_response_for_message(
    text: &str,
    incoming_chat: ChatId,
    allowed_chat: ChatId,
    control: &TelegramControl,
) -> Option<String> {
    if !is_authorized_chat(incoming_chat, allowed_chat) {
        return None;
    }

    match parse_command(text) {
        Ok(command) => Some(control.execute(command)),
        Err(CommandParseError::NotACommand) => None,
        Err(err) => Some(format!("Invalid command: {err}\n\n{}", command_help())),
    }
}

/// Check if a chat is authorized to send commands.
fn is_authorized_chat(incoming_chat: ChatId, allowed_chat: ChatId) -> bool {
    if incoming_chat == allowed_chat {
        return true;
    }

    warn!(
        chat_id = incoming_chat.0,
        "Ignoring Telegram message from unauthorized chat"
    );
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::inbound::runtime::{
        RuntimePosition, RuntimeRiskLimitKind, RuntimeRiskLimitUpdateError, RuntimeRiskLimits,
        RuntimeState,
    };
    use parking_lot::RwLock;
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    #[derive(Debug)]
    struct MockRuntimeState {
        limits: RwLock<RuntimeRiskLimits>,
        breaker_reason: RwLock<Option<String>>,
    }

    impl Default for MockRuntimeState {
        fn default() -> Self {
            Self {
                limits: RwLock::new(RuntimeRiskLimits {
                    max_position_per_market: dec!(100),
                    max_total_exposure: dec!(1000),
                    min_profit_threshold: dec!(0.2),
                    max_slippage: dec!(0.05),
                }),
                breaker_reason: RwLock::new(None),
            }
        }
    }

    impl RuntimeState for MockRuntimeState {
        fn risk_limits(&self) -> RuntimeRiskLimits {
            self.limits.read().clone()
        }

        fn set_risk_limit(
            &self,
            kind: RuntimeRiskLimitKind,
            value: rust_decimal::Decimal,
        ) -> Result<RuntimeRiskLimits, RuntimeRiskLimitUpdateError> {
            let mut limits = self.limits.write();
            match kind {
                RuntimeRiskLimitKind::MaxPositionPerMarket => {
                    limits.max_position_per_market = value
                }
                RuntimeRiskLimitKind::MaxTotalExposure => limits.max_total_exposure = value,
                RuntimeRiskLimitKind::MinProfitThreshold => limits.min_profit_threshold = value,
                RuntimeRiskLimitKind::MaxSlippage => limits.max_slippage = value,
            }
            Ok(limits.clone())
        }

        fn is_circuit_breaker_active(&self) -> bool {
            self.breaker_reason.read().is_some()
        }

        fn circuit_breaker_reason(&self) -> Option<String> {
            self.breaker_reason.read().clone()
        }

        fn activate_circuit_breaker(&self, reason: &str) {
            *self.breaker_reason.write() = Some(reason.to_string());
        }

        fn reset_circuit_breaker(&self) {
            *self.breaker_reason.write() = None;
        }

        fn open_position_count(&self) -> usize {
            0
        }

        fn total_exposure(&self) -> crate::domain::money::Price {
            rust_decimal::Decimal::ZERO
        }

        fn pending_exposure(&self) -> crate::domain::money::Price {
            rust_decimal::Decimal::ZERO
        }

        fn pending_execution_count(&self) -> usize {
            0
        }

        fn active_positions(&self) -> Vec<RuntimePosition> {
            Vec::new()
        }
    }

    fn as_runtime(state: Arc<MockRuntimeState>) -> Arc<dyn RuntimeState> {
        state
    }

    // -------------------------------------------------------------------------
    // Authorization tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_command_response_for_authorized_command() {
        let state = Arc::new(MockRuntimeState::default());
        let control = TelegramControl::new(as_runtime(state));
        let chat = ChatId(42);

        let response = command_response_for_message("/status", chat, chat, &control).unwrap();
        assert!(response.contains("Status"));
    }

    #[test]
    fn test_command_response_ignores_unauthorized_chat() {
        let state = Arc::new(MockRuntimeState::default());
        let control = TelegramControl::new(as_runtime(state));

        let response = command_response_for_message("/status", ChatId(7), ChatId(42), &control);
        assert!(response.is_none());
    }

    #[test]
    fn test_authorization_with_various_chat_ids() {
        let state = Arc::new(MockRuntimeState::default());
        let control = TelegramControl::new(as_runtime(state));
        let allowed = ChatId(12345);

        // Same chat ID should be authorized
        assert!(command_response_for_message("/status", allowed, allowed, &control).is_some());

        // Different chat IDs should be rejected
        assert!(command_response_for_message("/status", ChatId(1), allowed, &control).is_none());
        assert!(command_response_for_message("/status", ChatId(0), allowed, &control).is_none());
        assert!(command_response_for_message("/status", ChatId(-100), allowed, &control).is_none());
        assert!(
            command_response_for_message("/status", ChatId(99999), allowed, &control).is_none()
        );
    }

    #[test]
    fn test_authorization_with_negative_chat_ids() {
        // Telegram groups have negative chat IDs
        let state = Arc::new(MockRuntimeState::default());
        let control = TelegramControl::new(as_runtime(state));
        let allowed = ChatId(-123456789);

        // Same negative chat ID should be authorized
        assert!(command_response_for_message("/status", allowed, allowed, &control).is_some());

        // Different negative chat ID should be rejected
        assert!(
            command_response_for_message("/status", ChatId(-987654321), allowed, &control)
                .is_none()
        );
    }

    // -------------------------------------------------------------------------
    // Command parsing error handling
    // -------------------------------------------------------------------------

    #[test]
    fn test_command_response_for_invalid_command() {
        let state = Arc::new(MockRuntimeState::default());
        let control = TelegramControl::new(as_runtime(state));
        let chat = ChatId(42);

        let response = command_response_for_message("/bad", chat, chat, &control).unwrap();
        assert!(response.contains("Invalid command"));
        assert!(response.contains("Commands"));
    }

    #[test]
    fn test_command_response_ignores_non_command_text() {
        let state = Arc::new(MockRuntimeState::default());
        let control = TelegramControl::new(as_runtime(state));
        let chat = ChatId(42);

        let response = command_response_for_message("hello", chat, chat, &control);
        assert!(response.is_none());
    }

    #[test]
    fn test_command_response_ignores_empty_text() {
        let state = Arc::new(MockRuntimeState::default());
        let control = TelegramControl::new(as_runtime(state));
        let chat = ChatId(42);

        let response = command_response_for_message("", chat, chat, &control);
        assert!(response.is_none());
    }

    #[test]
    fn test_command_response_ignores_whitespace() {
        let state = Arc::new(MockRuntimeState::default());
        let control = TelegramControl::new(as_runtime(state));
        let chat = ChatId(42);

        let response = command_response_for_message("   ", chat, chat, &control);
        assert!(response.is_none());
    }

    // -------------------------------------------------------------------------
    // All commands execute correctly when authorized
    // -------------------------------------------------------------------------

    #[test]
    fn test_all_commands_execute_when_authorized() {
        let state = Arc::new(MockRuntimeState::default());
        let control = TelegramControl::new(as_runtime(state));
        let chat = ChatId(42);

        // Basic info commands
        assert!(command_response_for_message("/start", chat, chat, &control).is_some());
        assert!(command_response_for_message("/help", chat, chat, &control).is_some());
        assert!(command_response_for_message("/status", chat, chat, &control).is_some());
        assert!(command_response_for_message("/health", chat, chat, &control).is_some());
        assert!(command_response_for_message("/positions", chat, chat, &control).is_some());
        assert!(command_response_for_message("/version", chat, chat, &control).is_some());

        // Stats commands (may show "not available" without recorder)
        assert!(command_response_for_message("/stats", chat, chat, &control).is_some());
        assert!(command_response_for_message("/pool", chat, chat, &control).is_some());
        assert!(command_response_for_message("/markets", chat, chat, &control).is_some());

        // Control commands
        assert!(command_response_for_message("/pause", chat, chat, &control).is_some());
        assert!(command_response_for_message("/resume", chat, chat, &control).is_some());

        // Set risk command
        assert!(
            command_response_for_message("/set_risk min_profit 0.5", chat, chat, &control)
                .is_some()
        );
    }

    // -------------------------------------------------------------------------
    // Invalid command error messages
    // -------------------------------------------------------------------------

    #[test]
    fn test_unknown_command_shows_error_and_help() {
        let state = Arc::new(MockRuntimeState::default());
        let control = TelegramControl::new(as_runtime(state));
        let chat = ChatId(42);

        let response = command_response_for_message("/unknown", chat, chat, &control).unwrap();
        assert!(response.contains("Invalid command"));
        assert!(response.contains("unknown command"));
        assert!(response.contains("/status")); // Help should be included
    }

    #[test]
    fn test_set_risk_missing_args_shows_error() {
        let state = Arc::new(MockRuntimeState::default());
        let control = TelegramControl::new(as_runtime(state));
        let chat = ChatId(42);

        let response = command_response_for_message("/set_risk", chat, chat, &control).unwrap();
        assert!(response.contains("Invalid command"));
        assert!(response.contains("missing argument"));
    }

    #[test]
    fn test_set_risk_invalid_field_shows_error() {
        let state = Arc::new(MockRuntimeState::default());
        let control = TelegramControl::new(as_runtime(state));
        let chat = ChatId(42);

        let response =
            command_response_for_message("/set_risk bad_field 1.0", chat, chat, &control).unwrap();
        assert!(response.contains("Invalid command"));
        assert!(response.contains("invalid risk field"));
    }

    #[test]
    fn test_set_risk_invalid_value_shows_error() {
        let state = Arc::new(MockRuntimeState::default());
        let control = TelegramControl::new(as_runtime(state));
        let chat = ChatId(42);

        let response =
            command_response_for_message("/set_risk min_profit abc", chat, chat, &control).unwrap();
        assert!(response.contains("Invalid command"));
        assert!(response.contains("invalid decimal"));
    }

    // -------------------------------------------------------------------------
    // Command with bot mention
    // -------------------------------------------------------------------------

    #[test]
    fn test_command_with_bot_mention_authorized() {
        let state = Arc::new(MockRuntimeState::default());
        let control = TelegramControl::new(as_runtime(state));
        let chat = ChatId(42);

        let response =
            command_response_for_message("/status@my_bot", chat, chat, &control).unwrap();
        assert!(response.contains("Status"));
    }

    #[test]
    fn test_command_with_bot_mention_unauthorized() {
        let state = Arc::new(MockRuntimeState::default());
        let control = TelegramControl::new(as_runtime(state));

        let response =
            command_response_for_message("/status@my_bot", ChatId(7), ChatId(42), &control);
        assert!(response.is_none());
    }

    // -------------------------------------------------------------------------
    // Edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn test_message_starting_with_slash_but_not_command() {
        let state = Arc::new(MockRuntimeState::default());
        let control = TelegramControl::new(as_runtime(state));
        let chat = ChatId(42);

        // Just a slash
        let response = command_response_for_message("/", chat, chat, &control);
        assert!(response.is_some()); // Should be "unknown command"
        assert!(response.unwrap().contains("Invalid command"));
    }

    #[test]
    fn test_command_with_extra_whitespace() {
        let state = Arc::new(MockRuntimeState::default());
        let control = TelegramControl::new(as_runtime(state));
        let chat = ChatId(42);

        // Leading/trailing whitespace
        let response = command_response_for_message("  /status  ", chat, chat, &control).unwrap();
        assert!(response.contains("Status"));
    }

    #[test]
    fn test_commands_are_case_sensitive() {
        let state = Arc::new(MockRuntimeState::default());
        let control = TelegramControl::new(as_runtime(state));
        let chat = ChatId(42);

        // Uppercase should be treated as unknown command
        let response = command_response_for_message("/STATUS", chat, chat, &control).unwrap();
        assert!(response.contains("Invalid command"));
        assert!(response.contains("unknown command"));
    }

    // -------------------------------------------------------------------------
    // Concurrent access safety (basic test)
    // -------------------------------------------------------------------------

    #[test]
    fn test_multiple_commands_same_state() {
        let state = Arc::new(MockRuntimeState::default());
        let control = TelegramControl::new(as_runtime(Arc::clone(&state)));
        let chat = ChatId(42);

        // Execute multiple commands - they should all work
        for _ in 0..10 {
            assert!(command_response_for_message("/status", chat, chat, &control).is_some());
            assert!(command_response_for_message("/health", chat, chat, &control).is_some());
        }
    }
}
