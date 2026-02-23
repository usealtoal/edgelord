use std::path::Path;

use crate::adapter::inbound::cli::{operator, output};
use crate::error::Result;

/// Validate configuration file without starting the bot.
pub fn execute_config<P: AsRef<Path>>(config_path: P) -> Result<()> {
    let path = config_path.as_ref();
    let config_toml = operator::read_config_toml(path)?;
    let report = operator::operator().check_config(&config_toml)?;

    output::section("Configuration Check");
    output::field("Config", path.display());
    output::success("Configuration file is valid");

    output::section("Summary");
    output::field("Exchange", report.exchange);
    output::field("Environment", report.environment);
    output::field("Chain ID", report.chain_id);
    output::field("Strategies", format!("{:?}", report.enabled_strategies));
    output::field("Dry run", report.dry_run);

    if report.wallet_configured {
        output::success("Wallet credentials detected");
    } else {
        output::warning("Wallet credentials not configured (set WALLET_PRIVATE_KEY for trading)");
    }

    if report.telegram_enabled {
        if report.telegram_token_present && report.telegram_chat_present {
            output::success("Telegram integration configured");
        } else {
            output::warning("Telegram enabled but environment variables are missing");
            if !report.telegram_token_present {
                output::field("Missing", "TELEGRAM_BOT_TOKEN");
            }
            if !report.telegram_chat_present {
                output::field("Missing", "TELEGRAM_CHAT_ID");
            }
        }
    } else {
        output::field("Telegram", "disabled");
    }

    output::success("Configuration check complete");

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::port::inbound::operator::diagnostic::ConfigCheckReport;

    // Tests for ConfigCheckReport structure

    #[test]
    fn test_config_check_report_full_config() {
        let report = ConfigCheckReport {
            exchange: "polymarket".to_string(),
            environment: "mainnet".to_string(),
            chain_id: 137,
            enabled_strategies: vec!["binary".to_string(), "multi".to_string()],
            dry_run: false,
            wallet_configured: true,
            telegram_enabled: true,
            telegram_token_present: true,
            telegram_chat_present: true,
        };

        assert_eq!(report.exchange, "polymarket");
        assert_eq!(report.environment, "mainnet");
        assert_eq!(report.chain_id, 137);
        assert_eq!(report.enabled_strategies.len(), 2);
        assert!(!report.dry_run);
        assert!(report.wallet_configured);
        assert!(report.telegram_enabled);
        assert!(report.telegram_token_present);
        assert!(report.telegram_chat_present);
    }

    #[test]
    fn test_config_check_report_minimal_config() {
        let report = ConfigCheckReport {
            exchange: "".to_string(),
            environment: "".to_string(),
            chain_id: 0,
            enabled_strategies: vec![],
            dry_run: true,
            wallet_configured: false,
            telegram_enabled: false,
            telegram_token_present: false,
            telegram_chat_present: false,
        };

        assert!(report.exchange.is_empty());
        assert!(report.environment.is_empty());
        assert_eq!(report.chain_id, 0);
        assert!(report.enabled_strategies.is_empty());
        assert!(report.dry_run);
        assert!(!report.wallet_configured);
        assert!(!report.telegram_enabled);
    }

    // Tests for Telegram configuration states

    #[test]
    fn test_config_check_telegram_fully_configured() {
        let report = ConfigCheckReport {
            exchange: "polymarket".to_string(),
            environment: "mainnet".to_string(),
            chain_id: 137,
            enabled_strategies: vec![],
            dry_run: false,
            wallet_configured: true,
            telegram_enabled: true,
            telegram_token_present: true,
            telegram_chat_present: true,
        };

        // Both token and chat should be present
        assert!(report.telegram_enabled);
        assert!(report.telegram_token_present);
        assert!(report.telegram_chat_present);
    }

    #[test]
    fn test_config_check_telegram_missing_token() {
        let report = ConfigCheckReport {
            exchange: "polymarket".to_string(),
            environment: "mainnet".to_string(),
            chain_id: 137,
            enabled_strategies: vec![],
            dry_run: false,
            wallet_configured: true,
            telegram_enabled: true,
            telegram_token_present: false,
            telegram_chat_present: true,
        };

        assert!(report.telegram_enabled);
        assert!(!report.telegram_token_present);
        assert!(report.telegram_chat_present);
    }

    #[test]
    fn test_config_check_telegram_missing_chat() {
        let report = ConfigCheckReport {
            exchange: "polymarket".to_string(),
            environment: "mainnet".to_string(),
            chain_id: 137,
            enabled_strategies: vec![],
            dry_run: false,
            wallet_configured: true,
            telegram_enabled: true,
            telegram_token_present: true,
            telegram_chat_present: false,
        };

        assert!(report.telegram_enabled);
        assert!(report.telegram_token_present);
        assert!(!report.telegram_chat_present);
    }

    #[test]
    fn test_config_check_telegram_missing_both() {
        let report = ConfigCheckReport {
            exchange: "polymarket".to_string(),
            environment: "mainnet".to_string(),
            chain_id: 137,
            enabled_strategies: vec![],
            dry_run: false,
            wallet_configured: true,
            telegram_enabled: true,
            telegram_token_present: false,
            telegram_chat_present: false,
        };

        assert!(report.telegram_enabled);
        assert!(!report.telegram_token_present);
        assert!(!report.telegram_chat_present);
    }

    #[test]
    fn test_config_check_telegram_disabled() {
        let report = ConfigCheckReport {
            exchange: "polymarket".to_string(),
            environment: "mainnet".to_string(),
            chain_id: 137,
            enabled_strategies: vec![],
            dry_run: false,
            wallet_configured: true,
            telegram_enabled: false,
            telegram_token_present: false,
            telegram_chat_present: false,
        };

        assert!(!report.telegram_enabled);
    }

    // Tests for strategies formatting

    #[test]
    fn test_config_check_strategies_format_empty() {
        let report = ConfigCheckReport {
            exchange: "polymarket".to_string(),
            environment: "mainnet".to_string(),
            chain_id: 137,
            enabled_strategies: vec![],
            dry_run: false,
            wallet_configured: true,
            telegram_enabled: false,
            telegram_token_present: false,
            telegram_chat_present: false,
        };

        let formatted = format!("{:?}", report.enabled_strategies);
        assert_eq!(formatted, "[]");
    }

    #[test]
    fn test_config_check_strategies_format_single() {
        let report = ConfigCheckReport {
            exchange: "polymarket".to_string(),
            environment: "mainnet".to_string(),
            chain_id: 137,
            enabled_strategies: vec!["binary".to_string()],
            dry_run: false,
            wallet_configured: true,
            telegram_enabled: false,
            telegram_token_present: false,
            telegram_chat_present: false,
        };

        let formatted = format!("{:?}", report.enabled_strategies);
        assert_eq!(formatted, "[\"binary\"]");
    }

    #[test]
    fn test_config_check_strategies_format_multiple() {
        let report = ConfigCheckReport {
            exchange: "polymarket".to_string(),
            environment: "mainnet".to_string(),
            chain_id: 137,
            enabled_strategies: vec![
                "binary".to_string(),
                "multi".to_string(),
                "cluster".to_string(),
            ],
            dry_run: false,
            wallet_configured: true,
            telegram_enabled: false,
            telegram_token_present: false,
            telegram_chat_present: false,
        };

        let formatted = format!("{:?}", report.enabled_strategies);
        assert!(formatted.contains("binary"));
        assert!(formatted.contains("multi"));
        assert!(formatted.contains("cluster"));
    }

    // Tests for clone and debug

    #[test]
    fn test_config_check_report_clone() {
        let report = ConfigCheckReport {
            exchange: "polymarket".to_string(),
            environment: "mainnet".to_string(),
            chain_id: 137,
            enabled_strategies: vec!["binary".to_string()],
            dry_run: false,
            wallet_configured: true,
            telegram_enabled: true,
            telegram_token_present: true,
            telegram_chat_present: true,
        };

        let cloned = report.clone();
        assert_eq!(report.exchange, cloned.exchange);
        assert_eq!(report.environment, cloned.environment);
        assert_eq!(report.chain_id, cloned.chain_id);
        assert_eq!(report.enabled_strategies, cloned.enabled_strategies);
        assert_eq!(report.dry_run, cloned.dry_run);
        assert_eq!(report.wallet_configured, cloned.wallet_configured);
        assert_eq!(report.telegram_enabled, cloned.telegram_enabled);
        assert_eq!(report.telegram_token_present, cloned.telegram_token_present);
        assert_eq!(report.telegram_chat_present, cloned.telegram_chat_present);
    }

    #[test]
    fn test_config_check_report_debug() {
        let report = ConfigCheckReport {
            exchange: "polymarket".to_string(),
            environment: "mainnet".to_string(),
            chain_id: 137,
            enabled_strategies: vec!["binary".to_string()],
            dry_run: false,
            wallet_configured: true,
            telegram_enabled: false,
            telegram_token_present: false,
            telegram_chat_present: false,
        };

        let debug_str = format!("{:?}", report);
        assert!(debug_str.contains("ConfigCheckReport"));
        assert!(debug_str.contains("polymarket"));
        assert!(debug_str.contains("mainnet"));
        assert!(debug_str.contains("137"));
    }

    // Tests for chain ID variations

    #[test]
    fn test_config_check_polygon_mainnet_chain_id() {
        let report = ConfigCheckReport {
            exchange: "polymarket".to_string(),
            environment: "mainnet".to_string(),
            chain_id: 137,
            enabled_strategies: vec![],
            dry_run: false,
            wallet_configured: true,
            telegram_enabled: false,
            telegram_token_present: false,
            telegram_chat_present: false,
        };
        assert_eq!(report.chain_id, 137);
    }

    #[test]
    fn test_config_check_amoy_testnet_chain_id() {
        let report = ConfigCheckReport {
            exchange: "polymarket".to_string(),
            environment: "testnet".to_string(),
            chain_id: 80002,
            enabled_strategies: vec![],
            dry_run: true,
            wallet_configured: false,
            telegram_enabled: false,
            telegram_token_present: false,
            telegram_chat_present: false,
        };
        assert_eq!(report.chain_id, 80002);
    }

    // Tests for wallet configuration states

    #[test]
    fn test_config_check_wallet_configured() {
        let report = ConfigCheckReport {
            exchange: "polymarket".to_string(),
            environment: "mainnet".to_string(),
            chain_id: 137,
            enabled_strategies: vec![],
            dry_run: false,
            wallet_configured: true,
            telegram_enabled: false,
            telegram_token_present: false,
            telegram_chat_present: false,
        };
        assert!(report.wallet_configured);
    }

    #[test]
    fn test_config_check_wallet_not_configured() {
        let report = ConfigCheckReport {
            exchange: "polymarket".to_string(),
            environment: "testnet".to_string(),
            chain_id: 80002,
            enabled_strategies: vec![],
            dry_run: true,
            wallet_configured: false,
            telegram_enabled: false,
            telegram_token_present: false,
            telegram_chat_present: false,
        };
        assert!(!report.wallet_configured);
    }

    // Tests for dry run states

    #[test]
    fn test_config_check_dry_run_enabled() {
        let report = ConfigCheckReport {
            exchange: "polymarket".to_string(),
            environment: "testnet".to_string(),
            chain_id: 80002,
            enabled_strategies: vec![],
            dry_run: true,
            wallet_configured: false,
            telegram_enabled: false,
            telegram_token_present: false,
            telegram_chat_present: false,
        };
        assert!(report.dry_run);
    }

    #[test]
    fn test_config_check_dry_run_disabled() {
        let report = ConfigCheckReport {
            exchange: "polymarket".to_string(),
            environment: "mainnet".to_string(),
            chain_id: 137,
            enabled_strategies: vec![],
            dry_run: false,
            wallet_configured: true,
            telegram_enabled: false,
            telegram_token_present: false,
            telegram_chat_present: false,
        };
        assert!(!report.dry_run);
    }
}
