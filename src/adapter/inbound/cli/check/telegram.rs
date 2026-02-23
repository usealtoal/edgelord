use std::path::Path;

use crate::adapter::inbound::cli::{operator, output};
use crate::error::Result;
use serde_json::json;

/// Test Telegram notification by sending a test message.
pub async fn execute_telegram<P: AsRef<Path>>(config_path: P) -> Result<()> {
    let service = operator::operator();
    let config_toml = operator::read_config_toml(config_path.as_ref())?;
    let receipt = service.send_telegram_test(&config_toml).await?;

    if output::is_json() {
        output::json_output(json!({
            "command": "check.telegram",
            "masked_token": receipt.masked_token,
            "chat_id": receipt.chat_id,
            "status": "sent",
        }));
        return Ok(());
    }

    output::section("Telegram Check");
    output::action("Sending", "Telegram test message");
    output::field("Bot token", receipt.masked_token);
    output::field("Chat ID", &receipt.chat_id);
    output::action_done("Sent", "Telegram test message");
    output::hint("check Telegram for the message");

    Ok(())
}
