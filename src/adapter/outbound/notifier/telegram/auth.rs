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
}
