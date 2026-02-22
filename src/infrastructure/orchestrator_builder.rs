//! Registry and executor initialization.

use std::sync::Arc;

use chrono::Duration;
use tracing::{info, warn};

use super::config::{Config, LlmProvider};
use super::state::AppState;
use crate::application::inference::LlmInferrer;
use crate::adapter::llm::{Anthropic, Llm, OpenAi};
use crate::adapter::notifier::{LogNotifier, NotifierRegistry};
use crate::adapter::strategy::StrategyRegistry;
use crate::port::{ArbitrageExecutor, RelationInferrer};
use crate::infrastructure::cache::ClusterCache;
use crate::infrastructure::exchange::ExchangeFactory;

#[cfg(feature = "telegram")]
use crate::adapter::notifier::{RuntimeStats, TelegramConfig, TelegramNotifier};
#[cfg(feature = "telegram")]
use crate::application::statistic::StatsRecorder;

#[cfg(not(feature = "telegram"))]
use crate::application::statistic::StatsRecorder;

/// Build notifier registry from configuration.
///
/// When the `telegram` feature is enabled, this also creates a `RuntimeStats`
/// instance that should be updated by the orchestrator with pool and market info.
#[cfg(feature = "telegram")]
pub(crate) fn build_notifier_registry(
    config: &Config,
    state: Arc<AppState>,
    stats_recorder: Arc<StatsRecorder>,
) -> (NotifierRegistry, Option<Arc<RuntimeStats>>) {
    let mut registry = NotifierRegistry::new();

    // Always add log notifier
    registry.register(Box::new(LogNotifier));

    // Add telegram notifier if configured
    let runtime_stats = if config.telegram.enabled {
        if let Some(tg_config) = TelegramConfig::from_env() {
            let tg_config = TelegramConfig {
                notify_opportunities: config.telegram.notify_opportunities,
                notify_executions: config.telegram.notify_executions,
                notify_risk_rejections: config.telegram.notify_risk_rejections,
                position_display_limit: config.telegram.position_display_limit,
                ..tg_config
            };
            let runtime_stats = Arc::new(RuntimeStats::new());
            registry.register(Box::new(TelegramNotifier::new_with_full_control(
                tg_config,
                Arc::clone(&state),
                stats_recorder,
                Arc::clone(&runtime_stats),
            )));
            info!("Telegram notifier enabled with full control");
            Some(runtime_stats)
        } else {
            warn!("Telegram enabled but TELEGRAM_BOT_TOKEN or TELEGRAM_CHAT_ID not set");
            None
        }
    } else {
        None
    };

    (registry, runtime_stats)
}

/// Build notifier registry from configuration (non-telegram variant).
#[cfg(not(feature = "telegram"))]
pub(crate) fn build_notifier_registry(
    _config: &Config,
    _state: Arc<AppState>,
    _stats_recorder: Arc<StatsRecorder>,
) -> (NotifierRegistry, Option<()>) {
    let mut registry = NotifierRegistry::new();
    registry.register(Box::new(LogNotifier));
    (registry, None)
}

/// Build strategy registry from configuration using builder pattern.
pub(crate) fn build_strategy_registry(
    config: &Config,
    cluster_cache: Arc<ClusterCache>,
) -> StrategyRegistry {
    let mut builder = StrategyRegistry::builder().cluster_cache(cluster_cache);

    for name in &config.strategies.enabled {
        match name.as_str() {
            "single_condition" => {
                builder = builder.single_condition(config.strategies.single_condition.clone());
            }
            "market_rebalancing" => {
                builder = builder.market_rebalancing(config.strategies.market_rebalancing.clone());
            }
            "combinatorial" => {
                builder = builder.combinatorial(config.strategies.combinatorial.clone());
            }
            unknown => {
                warn!(strategy = unknown, "Unknown strategy in config, skipping");
            }
        }
    }

    builder.build()
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
            Arc::new(Anthropic::new(
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
            Arc::new(OpenAi::new(
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
pub(crate) fn build_inferrer(config: &Config, llm: Arc<dyn Llm>) -> Arc<dyn RelationInferrer> {
    let ttl = Duration::seconds(config.inference.ttl_seconds as i64);
    Arc::new(LlmInferrer::new(llm, ttl))
}
