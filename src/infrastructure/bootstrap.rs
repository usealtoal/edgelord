//! Infrastructure bootstrap helpers for runtime wiring.

use std::sync::Arc;

use chrono::Duration;
use tracing::{info, warn};

use crate::adapter::outbound::inference::inferrer::LlmInferrer;
use crate::adapter::outbound::llm::anthropic::Anthropic;
use crate::adapter::outbound::llm::openai::OpenAi;
#[cfg(feature = "telegram")]
use crate::adapter::outbound::notifier::telegram::control::RuntimeStats;
#[cfg(feature = "telegram")]
use crate::adapter::outbound::notifier::telegram::notifier::{TelegramConfig, TelegramNotifier};
use crate::adapter::outbound::solver::highs::HiGHSSolver;
use crate::adapter::outbound::sqlite::database::connection::{create_pool, run_migrations};
use crate::adapter::outbound::sqlite::stats_recorder;
use crate::application::cache::cluster::ClusterCache;
use crate::application::solver::frank_wolfe::FrankWolfeConfig;
use crate::application::solver::projection::FrankWolfeProjectionSolver;
use crate::application::state::AppState;
use crate::application::strategy::registry::StrategyRegistry;
use crate::error::Result;
use crate::infrastructure::config::llm::LlmProvider;
use crate::infrastructure::config::settings::Config;
use crate::infrastructure::exchange::factory::ExchangeFactory;
use crate::port::outbound::exchange::ArbitrageExecutor;
use crate::port::outbound::inference::RelationInferrer;
use crate::port::outbound::llm::Llm;
use crate::port::outbound::notifier::{LogNotifier, NotifierRegistry};
use crate::port::outbound::solver::ProjectionSolver;
use crate::port::outbound::stats::StatsRecorder;

/// Build notifier registry from configuration.
///
/// When the `telegram` feature is enabled, this also creates a `RuntimeStats`
/// instance that should be updated by the orchestrator with pool and market info.
#[cfg(feature = "telegram")]
pub(crate) fn build_notifier_registry(
    config: &Config,
    state: Arc<AppState>,
    stats_recorder: Arc<dyn StatsRecorder>,
) -> (NotifierRegistry, Option<Arc<RuntimeStats>>) {
    let mut registry = NotifierRegistry::new();
    registry.register(Box::new(LogNotifier));

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
            let runtime: Arc<dyn crate::port::inbound::runtime::RuntimeState> = state.clone();
            registry.register(Box::new(TelegramNotifier::new_with_full_control(
                tg_config,
                runtime,
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
    _stats_recorder: Arc<dyn StatsRecorder>,
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
    let mut builder = StrategyRegistry::builder()
        .cluster_cache(cluster_cache)
        .projection_solver(build_projection_solver());

    for name in &config.strategies.enabled {
        let normalized = normalize_strategy_name(name);
        match normalized.as_str() {
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
                warn!(
                    strategy = name,
                    normalized_strategy = unknown,
                    "Unknown strategy in config, skipping"
                );
            }
        }
    }

    builder.build()
}

fn normalize_strategy_name(raw: &str) -> String {
    raw.trim().to_lowercase().replace('-', "_")
}

/// Initialize SQLite and return a stats recorder.
pub(crate) fn init_stats_recorder(config: &Config) -> Result<Arc<dyn StatsRecorder>> {
    let db_url = format!("sqlite://{}", config.database);
    let db_pool = create_pool(&db_url)?;
    run_migrations(&db_pool)?;
    Ok(stats_recorder::create_recorder(db_pool))
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
                Ok(key) => key,
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
                Ok(key) => key,
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

/// Build inference service adapter.
pub(crate) fn build_inferrer(config: &Config, llm: Arc<dyn Llm>) -> Arc<dyn RelationInferrer> {
    let ttl = Duration::seconds(config.inference.ttl_seconds as i64);
    Arc::new(LlmInferrer::new(llm, ttl))
}

/// Build the default projection solver for cluster/combinatorial detection.
pub(crate) fn build_projection_solver() -> Arc<dyn ProjectionSolver> {
    Arc::new(FrankWolfeProjectionSolver::new(
        FrankWolfeConfig::default(),
        Arc::new(HiGHSSolver::new()),
    ))
}
