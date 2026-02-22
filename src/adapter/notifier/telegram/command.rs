//! Telegram command parsing.

use std::str::FromStr;

use rust_decimal::Decimal;

use crate::infrastructure::RiskLimitKind;

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
    SetRisk { kind: RiskLimitKind, value: Decimal },
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

fn parse_risk_limit_kind(value: &str) -> Result<RiskLimitKind, CommandParseError> {
    match value {
        "min_profit" | "min_profit_threshold" => Ok(RiskLimitKind::MinProfitThreshold),
        "max_slippage" | "slippage" => Ok(RiskLimitKind::MaxSlippage),
        "max_position" | "max_position_per_market" => Ok(RiskLimitKind::MaxPositionPerMarket),
        "max_exposure" | "max_total_exposure" => Ok(RiskLimitKind::MaxTotalExposure),
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
    fn parse_command_with_bot_mention() {
        assert_eq!(
            parse_command("/positions@edgelord_bot").unwrap(),
            TelegramCommand::Positions
        );
    }

    #[test]
    fn parse_set_risk() {
        assert_eq!(
            parse_command("/set_risk min_profit 0.25").unwrap(),
            TelegramCommand::SetRisk {
                kind: RiskLimitKind::MinProfitThreshold,
                value: dec!(0.25),
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
    fn parse_set_risk_invalid_value() {
        assert!(matches!(
            parse_command("/set_risk min_profit nope"),
            Err(CommandParseError::InvalidDecimal(_))
        ));
    }

    #[test]
    fn bot_commands_has_all_commands() {
        let commands = bot_commands();
        assert!(commands.iter().any(|(c, _)| *c == "status"));
        assert!(commands.iter().any(|(c, _)| *c == "stats"));
        assert!(commands.iter().any(|(c, _)| *c == "pool"));
        assert!(commands.iter().any(|(c, _)| *c == "markets"));
        assert!(commands.iter().any(|(c, _)| *c == "version"));
    }
}
