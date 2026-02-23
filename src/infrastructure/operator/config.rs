//! Configuration operator implementation.

use crate::error::Result;
use crate::infrastructure::config;
use crate::port::inbound::operator::config::{
    ConfigClusterDetection, ConfigInference, ConfigRiskLimits, ConfigValidationReport, ConfigView,
    ConfigurationOperator,
};

use super::entry::Operator;

impl ConfigurationOperator for Operator {
    fn show_config(&self, config_toml: &str) -> Result<ConfigView> {
        let config = config::settings::Config::parse_toml(config_toml)?;
        let network = config.network();

        Ok(ConfigView {
            profile: format!("{:?}", config.profile),
            dry_run: config.dry_run,
            exchange: format!("{:?}", config.exchange),
            environment: network.environment.to_string(),
            chain_id: network.chain_id,
            ws_url: network.ws_url,
            api_url: network.api_url,
            enabled_strategies: config.strategies.enabled,
            risk: ConfigRiskLimits {
                max_position_per_market: config.risk.max_position_per_market,
                max_total_exposure: config.risk.max_total_exposure,
                min_profit_threshold: config.risk.min_profit_threshold,
                max_slippage: config.risk.max_slippage,
            },
            wallet_private_key_loaded: config.wallet.private_key.is_some(),
            telegram_enabled: config.telegram.enabled,
            llm_provider: format!("{:?}", config.llm.provider),
            inference: ConfigInference {
                enabled: config.inference.enabled,
                min_confidence: config.inference.min_confidence,
                ttl_seconds: config.inference.ttl_seconds,
            },
            cluster_detection: ConfigClusterDetection {
                enabled: config.cluster_detection.enabled,
                debounce_ms: config.cluster_detection.debounce_ms,
                min_gap: config.cluster_detection.min_gap,
            },
        })
    }

    fn validate_config(&self, config_toml: &str) -> Result<ConfigValidationReport> {
        let config = config::settings::Config::parse_toml(config_toml)?;
        let mut warnings = Vec::new();

        if config.wallet.private_key.is_none() {
            warnings.push("WALLET_PRIVATE_KEY not set (required for trading)".to_string());
        }

        if config.strategies.enabled.is_empty() {
            warnings.push("No strategies enabled".to_string());
        }

        if config.network().is_environment_mainnet() && config.dry_run {
            warnings.push("Mainnet configured but dry_run is enabled".to_string());
        }

        if config.inference.enabled {
            let has_api_key = match config.llm.provider {
                config::llm::LlmProvider::Anthropic => std::env::var("ANTHROPIC_API_KEY").is_ok(),
                config::llm::LlmProvider::OpenAi => std::env::var("OPENAI_API_KEY").is_ok(),
            };
            if !has_api_key {
                warnings.push("Inference enabled but LLM API key not set".to_string());
            }
        }

        Ok(ConfigValidationReport { warnings })
    }
}
