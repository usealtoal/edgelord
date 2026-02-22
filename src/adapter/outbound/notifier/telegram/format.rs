//! Message formatting for Telegram notifications.

use crate::port::outbound::notifier::Event;

use super::notifier::TelegramConfig;

/// Format an event into a Telegram message, or None if the event should be skipped.
pub fn format_event_message(event: &Event, config: &TelegramConfig) -> Option<String> {
    match event {
        Event::OpportunityDetected(e) if config.notify_opportunities => {
            let question = truncate(&e.question, 60);

            Some(format!(
                "🎯 *Opportunity Detected*\n\
                \n\
                📋 {}\n\
                📈 Edge: `{:.2}%`\n\
                💵 Volume: `${:.2}`\n\
                💰 Expected: `\\+${:.2}`",
                escape_markdown(&question),
                e.edge * rust_decimal::Decimal::from(100),
                e.volume,
                e.expected_profit
            ))
        }
        Event::ExecutionCompleted(e) if config.notify_executions => {
            let (emoji, title) = if e.success {
                ("✅", "Trade Executed")
            } else {
                ("❌", "Execution Failed")
            };

            let market_display = truncate(&e.market_id, 16);

            Some(format!(
                "{} *{}*\n\
                \n\
                📋 Market: `{}`\n\
                📝 {}",
                emoji,
                title,
                escape_markdown(&market_display),
                escape_markdown(&e.details)
            ))
        }
        Event::RiskRejected(e) if config.notify_risk_rejections => {
            let market_display = truncate(&e.market_id, 16);

            Some(format!(
                "⚠️ *Risk Check Failed*\n\
                \n\
                📋 Market: `{}`\n\
                🚫 Reason: {}",
                escape_markdown(&market_display),
                escape_markdown(&e.reason)
            ))
        }
        Event::CircuitBreakerActivated { reason } => Some(format!(
            "🛑 *Circuit Breaker Activated*\n\
            \n\
            ⚠️ Reason: {}\n\
            ⏸️ Trading halted",
            escape_markdown(reason)
        )),
        Event::CircuitBreakerReset => Some(
            "✅ *Circuit Breaker Reset*\n\
            \n\
            ▶️ Trading resumed"
                .to_string(),
        ),
        Event::DailySummary(e) => Some(format!(
            "📊 *Daily Summary — {}*\n\
            \n\
            🎯 Opportunities: `{}`\n\
            📈 Trades: `{}`\n\
            ✅ Successful: `{}`\n\
            💰 Profit: `\\+${:.2}`\n\
            💼 Exposure: `${:.2}`",
            escape_markdown(&e.date.to_string()),
            e.opportunities_detected,
            e.trades_executed,
            e.trades_successful,
            e.total_profit,
            e.current_exposure
        )),
        Event::RelationsDiscovered(e) => {
            if e.relations.is_empty() {
                return None;
            }

            let mut msg = format!(
                "🔗 *Relations Discovered*\n\
                \n\
                Found `{}` relation\\(s\\)\n",
                e.relations_count
            );

            for (i, rel) in e.relations.iter().take(5).enumerate() {
                let type_emoji = match rel.relation_type.as_str() {
                    "mutually_exclusive" => "🔀",
                    "implies" => "➡️",
                    "exactly_one" => "☝️",
                    _ => "🔗",
                };

                let type_display = match rel.relation_type.as_str() {
                    "mutually_exclusive" => "Mutually Exclusive",
                    "implies" => "Implies",
                    "exactly_one" => "Exactly One",
                    other => other,
                };

                msg.push_str(&format!(
                    "\n{}\\. {} *{}* \\({}%\\)\n",
                    i + 1,
                    type_emoji,
                    type_display,
                    (rel.confidence * 100.0) as u32
                ));

                for q in &rel.market_questions {
                    let q_truncated = truncate(q, 50);
                    msg.push_str(&format!("   • {}\n", escape_markdown(&q_truncated)));
                }

                if !rel.reasoning.is_empty() {
                    let reasoning = truncate(&rel.reasoning, 60);
                    msg.push_str(&format!("   💡 _{}_\n", escape_markdown(&reasoning)));
                }
            }

            if e.relations.len() > 5 {
                msg.push_str(&format!("\n\\.\\.\\.and {} more", e.relations.len() - 5));
            }

            Some(msg)
        }
        _ => None,
    }
}

/// Truncate a string with ellipsis (Unicode-safe).
pub fn truncate(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count > max_chars {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}

/// Escape special characters for Telegram `MarkdownV2`.
pub fn escape_markdown(text: &str) -> String {
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
}
