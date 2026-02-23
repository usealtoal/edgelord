//! Telegram command parsing.

use std::str::FromStr;

use rust_decimal::Decimal;

use crate::port::inbound::runtime::RuntimeRiskLimitKind;

/// Supported Telegram commands.
#[derive(Debug, Clone, PartialEq)]
pub enum TelegramCommand {
    Start,
    Help,
    Status,
    Health,
    Positions,
    Stats,
    Pool,
    Markets,
    Version,
    Pause,
    Resume,
    SetRisk {
        kind: RuntimeRiskLimitKind,
        value: Decimal,
    },
}

/// Parse error for Telegram command messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandParseError {
    NotACommand,
    UnknownCommand(String),
    MissingArgument(&'static str),
    InvalidRiskField(String),
    InvalidDecimal(String),
}

impl std::fmt::Display for CommandParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotACommand => write!(f, "message is not a command"),
            Self::UnknownCommand(cmd) => write!(f, "unknown command `{cmd}`"),
            Self::MissingArgument(name) => write!(f, "missing argument `{name}`"),
            Self::InvalidRiskField(field) => write!(
                f,
                "invalid risk field `{field}` (use: min_profit, max_slippage, max_position, max_exposure)"
            ),
            Self::InvalidDecimal(value) => write!(f, "invalid decimal value `{value}`"),
        }
    }
}

impl std::error::Error for CommandParseError {}

/// Parse a Telegram message into a bot command.
pub fn parse_command(text: &str) -> Result<TelegramCommand, CommandParseError> {
    let mut parts = text.split_whitespace();
    let Some(raw_command) = parts.next() else {
        return Err(CommandParseError::NotACommand);
    };
    if !raw_command.starts_with('/') {
        return Err(CommandParseError::NotACommand);
    }

    let command = raw_command
        .split_once('@')
        .map_or(raw_command, |(head, _)| head);

    match command {
        "/start" => Ok(TelegramCommand::Start),
        "/help" => Ok(TelegramCommand::Help),
        "/status" => Ok(TelegramCommand::Status),
        "/health" => Ok(TelegramCommand::Health),
        "/positions" => Ok(TelegramCommand::Positions),
        "/stats" => Ok(TelegramCommand::Stats),
        "/pool" => Ok(TelegramCommand::Pool),
        "/markets" => Ok(TelegramCommand::Markets),
        "/version" => Ok(TelegramCommand::Version),
        "/pause" => Ok(TelegramCommand::Pause),
        "/resume" => Ok(TelegramCommand::Resume),
        "/set_risk" => {
            let raw_field = parts
                .next()
                .ok_or(CommandParseError::MissingArgument("field"))?;
            let raw_value = parts
                .next()
                .ok_or(CommandParseError::MissingArgument("value"))?;

            let kind = parse_risk_limit_kind(raw_field)?;
            let value = Decimal::from_str(raw_value)
                .map_err(|_| CommandParseError::InvalidDecimal(raw_value.to_string()))?;

            Ok(TelegramCommand::SetRisk { kind, value })
        }
        other => Err(CommandParseError::UnknownCommand(other.to_string())),
    }
}

fn parse_risk_limit_kind(value: &str) -> Result<RuntimeRiskLimitKind, CommandParseError> {
    match value {
        "min_profit" | "min_profit_threshold" => Ok(RuntimeRiskLimitKind::MinProfitThreshold),
        "max_slippage" | "slippage" => Ok(RuntimeRiskLimitKind::MaxSlippage),
        "max_position" | "max_position_per_market" => {
            Ok(RuntimeRiskLimitKind::MaxPositionPerMarket)
        }
        "max_exposure" | "max_total_exposure" => Ok(RuntimeRiskLimitKind::MaxTotalExposure),
        _ => Err(CommandParseError::InvalidRiskField(value.to_string())),
    }
}

/// Help text returned by `/start` and `/help`.
#[must_use]
pub const fn command_help() -> &'static str {
    "📋 Commands\n\n\
    /status - 📊 Runtime status and configuration\n\
    /health - 🏥 System health check\n\
    /positions - 💼 Active positions\n\
    /stats - 📈 Today's trading statistics\n\
    /pool - 🔌 Connection pool status\n\
    /markets - 🏛️ Subscribed markets info\n\
    /version - 🔖 Build version\n\
    /pause - ⏸️ Halt trading\n\
    /resume - ▶️ Resume trading\n\
    /set_risk <field> <value> - ⚙️ Update risk limit\n\n\
    Risk fields: min_profit, max_slippage, max_position, max_exposure"
}

/// Bot commands for Telegram menu registration.
///
/// Returns tuples of (command, description) for `set_my_commands`.
#[must_use]
pub fn bot_commands() -> Vec<(&'static str, &'static str)> {
    vec![
        ("status", "Runtime status and configuration"),
        ("health", "System health check"),
        ("positions", "Active positions"),
        ("stats", "Today's trading statistics"),
        ("pool", "Connection pool status"),
        ("markets", "Subscribed markets info"),
        ("version", "Build version"),
        ("pause", "Halt trading"),
        ("resume", "Resume trading"),
        ("set_risk", "Update risk limit"),
        ("help", "Show all commands"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // -------------------------------------------------------------------------
    // Basic command parsing
    // -------------------------------------------------------------------------

    #[test]
    fn parse_known_command() {
        assert_eq!(parse_command("/status").unwrap(), TelegramCommand::Status);
    }

    #[test]
    fn parse_new_commands() {
        assert_eq!(parse_command("/stats").unwrap(), TelegramCommand::Stats);
        assert_eq!(parse_command("/pool").unwrap(), TelegramCommand::Pool);
        assert_eq!(parse_command("/markets").unwrap(), TelegramCommand::Markets);
        assert_eq!(parse_command("/version").unwrap(), TelegramCommand::Version);
    }

    #[test]
    fn parse_all_basic_commands() {
        assert_eq!(parse_command("/start").unwrap(), TelegramCommand::Start);
        assert_eq!(parse_command("/help").unwrap(), TelegramCommand::Help);
        assert_eq!(parse_command("/status").unwrap(), TelegramCommand::Status);
        assert_eq!(parse_command("/health").unwrap(), TelegramCommand::Health);
        assert_eq!(
            parse_command("/positions").unwrap(),
            TelegramCommand::Positions
        );
        assert_eq!(parse_command("/stats").unwrap(), TelegramCommand::Stats);
        assert_eq!(parse_command("/pool").unwrap(), TelegramCommand::Pool);
        assert_eq!(parse_command("/markets").unwrap(), TelegramCommand::Markets);
        assert_eq!(parse_command("/version").unwrap(), TelegramCommand::Version);
        assert_eq!(parse_command("/pause").unwrap(), TelegramCommand::Pause);
        assert_eq!(parse_command("/resume").unwrap(), TelegramCommand::Resume);
    }

    // -------------------------------------------------------------------------
    // Bot mention handling (commands with @bot_name suffix)
    // -------------------------------------------------------------------------

    #[test]
    fn parse_command_with_bot_mention() {
        assert_eq!(
            parse_command("/positions@edgelord_bot").unwrap(),
            TelegramCommand::Positions
        );
    }

    #[test]
    fn parse_command_with_different_bot_mentions() {
        assert_eq!(
            parse_command("/status@mybot").unwrap(),
            TelegramCommand::Status
        );
        assert_eq!(
            parse_command("/help@another_bot_123").unwrap(),
            TelegramCommand::Help
        );
        assert_eq!(parse_command("/pause@BOT").unwrap(), TelegramCommand::Pause);
    }

    // -------------------------------------------------------------------------
    // Set risk command parsing
    // -------------------------------------------------------------------------

    #[test]
    fn parse_set_risk() {
        assert_eq!(
            parse_command("/set_risk min_profit 0.25").unwrap(),
            TelegramCommand::SetRisk {
                kind: RuntimeRiskLimitKind::MinProfitThreshold,
                value: dec!(0.25),
            }
        );
    }

    #[test]
    fn parse_set_risk_all_fields() {
        // min_profit aliases
        assert_eq!(
            parse_command("/set_risk min_profit 0.5").unwrap(),
            TelegramCommand::SetRisk {
                kind: RuntimeRiskLimitKind::MinProfitThreshold,
                value: dec!(0.5),
            }
        );
        assert_eq!(
            parse_command("/set_risk min_profit_threshold 1.0").unwrap(),
            TelegramCommand::SetRisk {
                kind: RuntimeRiskLimitKind::MinProfitThreshold,
                value: dec!(1.0),
            }
        );

        // max_slippage aliases
        assert_eq!(
            parse_command("/set_risk max_slippage 0.1").unwrap(),
            TelegramCommand::SetRisk {
                kind: RuntimeRiskLimitKind::MaxSlippage,
                value: dec!(0.1),
            }
        );
        assert_eq!(
            parse_command("/set_risk slippage 0.05").unwrap(),
            TelegramCommand::SetRisk {
                kind: RuntimeRiskLimitKind::MaxSlippage,
                value: dec!(0.05),
            }
        );

        // max_position aliases
        assert_eq!(
            parse_command("/set_risk max_position 100").unwrap(),
            TelegramCommand::SetRisk {
                kind: RuntimeRiskLimitKind::MaxPositionPerMarket,
                value: dec!(100),
            }
        );
        assert_eq!(
            parse_command("/set_risk max_position_per_market 200").unwrap(),
            TelegramCommand::SetRisk {
                kind: RuntimeRiskLimitKind::MaxPositionPerMarket,
                value: dec!(200),
            }
        );

        // max_exposure aliases
        assert_eq!(
            parse_command("/set_risk max_exposure 1000").unwrap(),
            TelegramCommand::SetRisk {
                kind: RuntimeRiskLimitKind::MaxTotalExposure,
                value: dec!(1000),
            }
        );
        assert_eq!(
            parse_command("/set_risk max_total_exposure 5000").unwrap(),
            TelegramCommand::SetRisk {
                kind: RuntimeRiskLimitKind::MaxTotalExposure,
                value: dec!(5000),
            }
        );
    }

    #[test]
    fn parse_set_risk_decimal_values() {
        // Integer
        assert_eq!(
            parse_command("/set_risk max_position 100").unwrap(),
            TelegramCommand::SetRisk {
                kind: RuntimeRiskLimitKind::MaxPositionPerMarket,
                value: dec!(100),
            }
        );

        // Simple decimal
        assert_eq!(
            parse_command("/set_risk min_profit 0.25").unwrap(),
            TelegramCommand::SetRisk {
                kind: RuntimeRiskLimitKind::MinProfitThreshold,
                value: dec!(0.25),
            }
        );

        // Many decimal places
        assert_eq!(
            parse_command("/set_risk max_slippage 0.001").unwrap(),
            TelegramCommand::SetRisk {
                kind: RuntimeRiskLimitKind::MaxSlippage,
                value: dec!(0.001),
            }
        );

        // Large number
        assert_eq!(
            parse_command("/set_risk max_exposure 999999").unwrap(),
            TelegramCommand::SetRisk {
                kind: RuntimeRiskLimitKind::MaxTotalExposure,
                value: dec!(999999),
            }
        );

        // Negative (parser accepts, validation happens elsewhere)
        assert_eq!(
            parse_command("/set_risk min_profit -1").unwrap(),
            TelegramCommand::SetRisk {
                kind: RuntimeRiskLimitKind::MinProfitThreshold,
                value: dec!(-1),
            }
        );
    }

    #[test]
    fn parse_set_risk_invalid_field() {
        assert!(matches!(
            parse_command("/set_risk unknown 1"),
            Err(CommandParseError::InvalidRiskField(_))
        ));
    }

    #[test]
    fn parse_set_risk_various_invalid_fields() {
        let invalid_fields = [
            "profit",
            "slippage_max",
            "position",
            "exposure",
            "foo",
            "123",
            "",
        ];

        for field in invalid_fields {
            let cmd = format!("/set_risk {} 1", field);
            let result = parse_command(&cmd);
            assert!(
                matches!(result, Err(CommandParseError::InvalidRiskField(_)))
                    || matches!(result, Err(CommandParseError::MissingArgument(_))),
                "Expected InvalidRiskField or MissingArgument for field '{}', got {:?}",
                field,
                result
            );
        }
    }

    #[test]
    fn parse_set_risk_invalid_value() {
        assert!(matches!(
            parse_command("/set_risk min_profit nope"),
            Err(CommandParseError::InvalidDecimal(_))
        ));
    }

    #[test]
    fn parse_set_risk_various_invalid_values() {
        let invalid_values = ["abc", "12.34.56", "1e10", "NaN", "inf", ""];

        for value in invalid_values {
            let cmd = format!("/set_risk min_profit {}", value);
            let result = parse_command(&cmd);
            assert!(
                matches!(result, Err(CommandParseError::InvalidDecimal(_)))
                    || matches!(result, Err(CommandParseError::MissingArgument(_))),
                "Expected InvalidDecimal or MissingArgument for value '{}', got {:?}",
                value,
                result
            );
        }
    }

    #[test]
    fn parse_set_risk_missing_field() {
        assert!(matches!(
            parse_command("/set_risk"),
            Err(CommandParseError::MissingArgument("field"))
        ));
    }

    #[test]
    fn parse_set_risk_missing_value() {
        assert!(matches!(
            parse_command("/set_risk min_profit"),
            Err(CommandParseError::MissingArgument("value"))
        ));
    }

    #[test]
    fn parse_set_risk_with_bot_mention() {
        assert_eq!(
            parse_command("/set_risk@mybot min_profit 0.5").unwrap(),
            TelegramCommand::SetRisk {
                kind: RuntimeRiskLimitKind::MinProfitThreshold,
                value: dec!(0.5),
            }
        );
    }

    #[test]
    fn parse_set_risk_extra_arguments_ignored() {
        // Extra arguments after the required ones should be ignored
        let result = parse_command("/set_risk min_profit 0.5 extra ignored");
        assert_eq!(
            result.unwrap(),
            TelegramCommand::SetRisk {
                kind: RuntimeRiskLimitKind::MinProfitThreshold,
                value: dec!(0.5),
            }
        );
    }

    // -------------------------------------------------------------------------
    // Error cases
    // -------------------------------------------------------------------------

    #[test]
    fn parse_not_a_command() {
        assert!(matches!(
            parse_command("hello"),
            Err(CommandParseError::NotACommand)
        ));
    }

    #[test]
    fn parse_empty_string() {
        assert!(matches!(
            parse_command(""),
            Err(CommandParseError::NotACommand)
        ));
    }

    #[test]
    fn parse_whitespace_only() {
        assert!(matches!(
            parse_command("   "),
            Err(CommandParseError::NotACommand)
        ));
    }

    #[test]
    fn parse_unknown_command() {
        let err = parse_command("/unknown").unwrap_err();
        assert!(matches!(err, CommandParseError::UnknownCommand(ref cmd) if cmd == "/unknown"));
    }

    #[test]
    fn parse_various_unknown_commands() {
        let unknown = ["/foo", "/bar", "/test", "/123", "/a"];

        for cmd in unknown {
            let err = parse_command(cmd).unwrap_err();
            assert!(
                matches!(err, CommandParseError::UnknownCommand(_)),
                "Expected UnknownCommand for '{}', got {:?}",
                cmd,
                err
            );
        }
    }

    #[test]
    fn parse_slash_only() {
        let err = parse_command("/").unwrap_err();
        assert!(matches!(err, CommandParseError::UnknownCommand(ref cmd) if cmd == "/"));
    }

    // -------------------------------------------------------------------------
    // Edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn parse_command_with_leading_whitespace() {
        // Leading whitespace should be ignored by split_whitespace
        assert_eq!(parse_command("  /status").unwrap(), TelegramCommand::Status);
    }

    #[test]
    fn parse_command_with_trailing_whitespace() {
        assert_eq!(
            parse_command("/status   ").unwrap(),
            TelegramCommand::Status
        );
    }

    #[test]
    fn parse_command_case_sensitivity() {
        // Commands are case-sensitive (lowercase only)
        assert!(matches!(
            parse_command("/STATUS"),
            Err(CommandParseError::UnknownCommand(_))
        ));
        assert!(matches!(
            parse_command("/Status"),
            Err(CommandParseError::UnknownCommand(_))
        ));
        assert!(matches!(
            parse_command("/PAUSE"),
            Err(CommandParseError::UnknownCommand(_))
        ));
    }

    #[test]
    fn parse_command_with_tabs() {
        // Tabs as whitespace should work
        assert_eq!(
            parse_command("/set_risk\tmin_profit\t0.5").unwrap(),
            TelegramCommand::SetRisk {
                kind: RuntimeRiskLimitKind::MinProfitThreshold,
                value: dec!(0.5),
            }
        );
    }

    #[test]
    fn parse_command_with_mixed_whitespace() {
        assert_eq!(
            parse_command("/set_risk  min_profit   0.5").unwrap(),
            TelegramCommand::SetRisk {
                kind: RuntimeRiskLimitKind::MinProfitThreshold,
                value: dec!(0.5),
            }
        );
    }

    // -------------------------------------------------------------------------
    // Bot commands registration
    // -------------------------------------------------------------------------

    #[test]
    fn bot_commands_has_all_commands() {
        let commands = bot_commands();
        assert!(commands.iter().any(|(c, _)| *c == "status"));
        assert!(commands.iter().any(|(c, _)| *c == "stats"));
        assert!(commands.iter().any(|(c, _)| *c == "pool"));
        assert!(commands.iter().any(|(c, _)| *c == "markets"));
        assert!(commands.iter().any(|(c, _)| *c == "version"));
    }

    #[test]
    fn bot_commands_complete() {
        let commands = bot_commands();
        let expected = [
            "status",
            "health",
            "positions",
            "stats",
            "pool",
            "markets",
            "version",
            "pause",
            "resume",
            "set_risk",
            "help",
        ];

        for cmd in expected {
            assert!(
                commands.iter().any(|(c, _)| *c == cmd),
                "Missing command: {}",
                cmd
            );
        }

        assert_eq!(commands.len(), expected.len());
    }

    #[test]
    fn bot_commands_have_descriptions() {
        let commands = bot_commands();
        for (cmd, desc) in &commands {
            assert!(!cmd.is_empty(), "Empty command name");
            assert!(!desc.is_empty(), "Empty description for command: {}", cmd);
        }
    }

    // -------------------------------------------------------------------------
    // Command help
    // -------------------------------------------------------------------------

    #[test]
    fn command_help_contains_all_commands() {
        let help = command_help();
        assert!(help.contains("/status"));
        assert!(help.contains("/health"));
        assert!(help.contains("/positions"));
        assert!(help.contains("/stats"));
        assert!(help.contains("/pool"));
        assert!(help.contains("/markets"));
        assert!(help.contains("/version"));
        assert!(help.contains("/pause"));
        assert!(help.contains("/resume"));
        assert!(help.contains("/set_risk"));
    }

    #[test]
    fn command_help_contains_risk_fields() {
        let help = command_help();
        assert!(help.contains("min_profit"));
        assert!(help.contains("max_slippage"));
        assert!(help.contains("max_position"));
        assert!(help.contains("max_exposure"));
    }

    // -------------------------------------------------------------------------
    // CommandParseError Display
    // -------------------------------------------------------------------------

    #[test]
    fn command_parse_error_display() {
        assert_eq!(
            CommandParseError::NotACommand.to_string(),
            "message is not a command"
        );

        assert_eq!(
            CommandParseError::UnknownCommand("/foo".to_string()).to_string(),
            "unknown command `/foo`"
        );

        assert_eq!(
            CommandParseError::MissingArgument("field").to_string(),
            "missing argument `field`"
        );

        assert_eq!(
            CommandParseError::InvalidRiskField("bad".to_string()).to_string(),
            "invalid risk field `bad` (use: min_profit, max_slippage, max_position, max_exposure)"
        );

        assert_eq!(
            CommandParseError::InvalidDecimal("xyz".to_string()).to_string(),
            "invalid decimal value `xyz`"
        );
    }
}
