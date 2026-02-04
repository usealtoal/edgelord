//! Registry and executor initialization.

use std::sync::Arc;

use chrono::Duration;
use tracing::{info, warn};

use crate::app::config::{Config, LlmProvider};
use crate::core::cache::ClusterCache;
use crate::core::exchange::{ArbitrageExecutor, ExchangeFactory};
use crate::core::llm::{AnthropicLlm, Llm, OpenAiLlm};
use crate::core::service::inference::{Inferrer, LlmInferrer};
use crate::core::service::{LogNotifier, NotifierRegistry};
use crate::core::strategy::{
    CombinatorialStrategy, MarketRebalancingStrategy, SingleConditionStrategy, StrategyRegistry,
};

#[cfg(feature = "telegram")]
use crate::core::service::{TelegramConfig, TelegramNotifier};

/// Build notifier registry from configuration.
pub(crate) fn build_notifier_registry(config: &Config) -> NotifierRegistry {
    let mut registry = NotifierRegistry::new();

    // Always add log notifier
    registry.register(Box::new(LogNotifier));

    // Add telegram notifier if configured
    #[cfg(feature = "telegram")]
    if config.telegram.enabled {
        if let Some(tg_config) = TelegramConfig::from_env() {
            let tg_config = TelegramConfig {
                notify_opportunities: config.telegram.notify_opportunities,
                notify_executions: config.telegram.notify_executions,
                notify_risk_rejections: config.telegram.notify_risk_rejections,
                ..tg_config
            };
            registry.register(Box::new(TelegramNotifier::new(tg_config)));
            info!("Telegram notifier enabled");
        } else {
            warn!("Telegram enabled but TELEGRAM_BOT_TOKEN or TELEGRAM_CHAT_ID not set");
        }
    }

    // Suppress unused variable warning when telegram feature is disabled
    #[cfg(not(feature = "telegram"))]
    let _ = config;

    registry
}

/// Build strategy registry from configuration.
pub(crate) fn build_strategy_registry(config: &Config) -> StrategyRegistry {
    let mut registry = StrategyRegistry::new();

    for name in &config.strategies.enabled {
        match name.as_str() {
            "single_condition" => {
                registry.register(Box::new(SingleConditionStrategy::new(
                    config.strategies.single_condition.clone(),
                )));
            }
            "market_rebalancing" => {
                registry.register(Box::new(MarketRebalancingStrategy::new(
                    config.strategies.market_rebalancing.clone(),
                )));
            }
            "combinatorial" => {
                if config.strategies.combinatorial.enabled {
                    registry.register(Box::new(CombinatorialStrategy::new(
                        config.strategies.combinatorial.clone(),
                    )));
                }
            }
            unknown => {
                warn!(strategy = unknown, "Unknown strategy in config, skipping");
            }
        }
    }

    registry
}

/// Initialize the executor if wallet is configured.
pub(crate) async fn init_executor(
    config: &Config,
) -> Option<Arc<dyn ArbitrageExecutor + Send + Sync>> {
    match ExchangeFactory::create_arbitrage_executor(config).await {
        Ok(Some(exec)) => {
            info!("Executor initialized - trading ENABLED");
            Some(exec)
        }
        Ok(None) => {
            info!("No wallet configured - detection only mode");
            None
        }
        Err(e) => {
            warn!(error = %e, "Failed to initialize executor - detection only");
            None
        }
    }
}

/// Build LLM client from configuration.
pub(crate) fn build_llm_client(config: &Config) -> Option<Arc<dyn Llm>> {
    if !config.inference.enabled {
        return None;
    }

    let client: Arc<dyn Llm> = match config.llm.provider {
        LlmProvider::Anthropic => {
            let api_key = match std::env::var("ANTHROPIC_API_KEY") {
                Ok(k) => k,
                Err(_) => {
                    warn!("ANTHROPIC_API_KEY not set, inference disabled");
                    return None;
                }
            };
            Arc::new(AnthropicLlm::new(
                api_key,
                &config.llm.anthropic.model,
                config.llm.anthropic.max_tokens,
                config.llm.anthropic.temperature,
            ))
        }
        LlmProvider::OpenAi => {
            let api_key = match std::env::var("OPENAI_API_KEY") {
                Ok(k) => k,
                Err(_) => {
                    warn!("OPENAI_API_KEY not set, inference disabled");
                    return None;
                }
            };
            Arc::new(OpenAiLlm::new(
                api_key,
                &config.llm.openai.model,
                config.llm.openai.max_tokens,
                config.llm.openai.temperature,
            ))
        }
    };

    info!(provider = client.name(), "LLM client initialized");
    Some(client)
}

/// Build cluster cache for relation inference.
pub(crate) fn build_cluster_cache(config: &Config) -> Arc<ClusterCache> {
    let ttl = Duration::seconds(config.inference.ttl_seconds as i64);
    Arc::new(ClusterCache::new(ttl))
}

/// Build inference service components.
pub(crate) fn build_inferrer(
    config: &Config,
    llm: Arc<dyn Llm>,
) -> Arc<dyn Inferrer> {
    let ttl = Duration::seconds(config.inference.ttl_seconds as i64);
    Arc::new(LlmInferrer::new(llm, ttl))
}
