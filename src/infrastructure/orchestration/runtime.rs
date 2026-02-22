//! Orchestrator runtime lifecycle.

use std::sync::Arc;

use tokio::sync::watch;
use tracing::{info, warn};

use super::cluster;
use super::context::EventProcessingContext;
use super::inference;
use super::orchestrator::{process_market_event, Orchestrator};
use super::startup;
use super::stream;
use crate::application::position::manager::PositionManager;
use crate::application::risk::manager::RiskManager;
use crate::application::state::AppState;
use crate::error::Result;
use crate::infrastructure::bootstrap::{
    build_cluster_cache, build_inferrer, build_llm_client, build_notifier_registry,
    build_strategy_registry, init_executor, init_stats_recorder,
};
use crate::infrastructure::config::settings::Config;
#[cfg(feature = "telegram")]
use crate::port::inbound::runtime::RuntimeClusterView;
use crate::port::outbound::inference::RelationInferrer;

impl Orchestrator {
    /// Run the main application loop.
    pub async fn run(config: Config) -> Result<()> {
        let (_shutdown_tx, shutdown_rx) = watch::channel(false);
        super::runtime::run_with_shutdown(config, shutdown_rx).await
    }

    /// Run with externally controlled shutdown signal.
    pub async fn run_with_shutdown(config: Config, shutdown: watch::Receiver<bool>) -> Result<()> {
        super::runtime::run_with_shutdown(config, shutdown).await
    }
}

/// Runtime loop entrypoint used by [`Orchestrator`].
pub async fn run_with_shutdown(config: Config, mut shutdown: watch::Receiver<bool>) -> Result<()> {
    info!(exchange = ?config.exchange, dry_run = config.dry_run, "Starting edgelord");

    let state = Arc::new(AppState::new(config.risk.clone().into()));
    let stats_recorder = init_stats_recorder(&config)?;
    let position_manager = Arc::new(PositionManager::new(Arc::clone(&stats_recorder)));
    info!(database = %config.database, "Database initialized");

    let risk_manager = Arc::new(RiskManager::new(state.clone()));

    #[allow(unused_variables)]
    let (notifiers, runtime_stats) =
        build_notifier_registry(&config, Arc::clone(&state), Arc::clone(&stats_recorder));
    let notifiers = Arc::new(notifiers);
    info!(notifiers = notifiers.len(), "Notifiers initialized");

    let executor = init_executor(&config).await;

    let cluster_cache = build_cluster_cache(&config);
    let llm_client = build_llm_client(&config);
    let inferrer: Option<Arc<dyn RelationInferrer>> =
        llm_client.map(|llm| build_inferrer(&config, llm));
    if inferrer.is_some() {
        info!("Inference service enabled");
    }

    #[cfg(feature = "telegram")]
    if let Some(ref stats) = runtime_stats {
        stats.set_cluster_cache(Arc::clone(&cluster_cache) as Arc<dyn RuntimeClusterView>);
    }

    let strategies = build_strategy_registry(&config, Arc::clone(&cluster_cache));
    let Some(prepared) = startup::prepare_markets(&config, strategies).await? else {
        return Ok(());
    };

    #[cfg(feature = "telegram")]
    if let Some(ref stats) = runtime_stats {
        stats.update_market_counts(prepared.registry.len(), prepared.token_ids.len());
    }

    inference::run_startup_inference(
        &config,
        inferrer.as_ref(),
        &prepared.market_summaries,
        cluster_cache.as_ref(),
        &notifiers,
    )
    .await;

    let (cache, cluster_handle) = cluster::setup_cluster_detection(
        &config,
        Arc::clone(&prepared.registry),
        Arc::clone(&cluster_cache),
        Arc::clone(&notifiers),
    );

    let _inference_handle = inference::start_continuous_inference(
        &config,
        inferrer,
        Arc::clone(&cluster_cache),
        Arc::new(prepared.market_summaries.clone()),
    );

    let mut data_stream = stream::create_connected_stream(&config, &prepared.token_ids).await?;
    info!("Listening for market events...");

    let dry_run = config.dry_run;
    let stats_interval_secs = config.telegram.stats_interval_secs;
    let mut stats_interval =
        tokio::time::interval(std::time::Duration::from_secs(stats_interval_secs));
    stats_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            result = shutdown.changed() => {
                match result {
                    Ok(_) => {
                        if *shutdown.borrow() {
                            info!("Shutdown signal received");
                            break;
                        }
                    }
                    Err(_) => {
                        info!("Shutdown channel closed");
                        break;
                    }
                }
            }
            _ = stats_interval.tick() => {
                #[cfg(feature = "telegram")]
                if let Some(ref stats) = runtime_stats {
                    if let Some(pool_stats) = data_stream.pool_stats() {
                        stats.update_pool_stats(pool_stats);
                    }
                }
            }
            event = data_stream.next_event() => {
                let Some(event) = event else {
                    warn!("Market data stream ended");
                    break;
                };
                process_market_event(
                    event,
                    EventProcessingContext {
                        cache: &cache,
                        registry: &prepared.registry,
                        strategies: prepared.strategies.as_ref(),
                        executor: executor.clone(),
                        risk_manager: &risk_manager,
                        notifiers: &notifiers,
                        state: &state,
                        stats: &stats_recorder,
                        position_manager: &position_manager,
                        dry_run,
                    },
                );
            }
        }
    }

    if let Some(handle) = cluster_handle {
        handle.shutdown().await;
    }

    Ok(())
}
