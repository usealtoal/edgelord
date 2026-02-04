//! App orchestration module.
//!
//! This module contains the main application logic for running
//! the edgelord arbitrage detection and execution system.

// Allow many arguments for handler functions that coordinate multiple services
#![allow(clippy::too_many_arguments)]

mod builder;
mod execution;
mod handler;

use std::sync::Arc;

use tracing::{debug, info, warn};

use crate::app::config::Config;
use crate::app::state::AppState;
use crate::app::status::{StatusConfig, StatusWriter};
use crate::core::cache::OrderBookCache;
use crate::core::domain::{MarketRegistry, TokenId};
use crate::core::exchange::{ExchangeFactory, MarketDataStream, ReconnectingDataStream};
use crate::core::service::inference::{Inferrer, MarketSummary};
use crate::core::service::RiskManager;
use crate::error::Result;

use builder::{
    build_cluster_cache, build_inferrer, build_llm_client, build_notifier_registry,
    build_strategy_registry, init_executor,
};
use handler::handle_market_event;

/// Main application orchestrator.
pub(crate) struct Orchestrator;

impl Orchestrator {
    /// Run the main application loop.
    pub async fn run(config: Config) -> Result<()> {
        info!(exchange = ?config.exchange, dry_run = config.dry_run, "Starting edgelord");

        // Initialize shared state
        let state = Arc::new(AppState::new(config.risk.clone().into()));

        // Initialize risk manager
        let risk_manager = Arc::new(RiskManager::new(state.clone()));

        // Initialize notifiers
        let notifiers = Arc::new(build_notifier_registry(&config));
        info!(notifiers = notifiers.len(), "Notifiers initialized");

        // Initialize status writer if configured
        let status_writer = config.status_file.as_ref().map(|path| {
            let network = config.network();
            let status_config = StatusConfig {
                exchange: format!("{:?}", config.exchange).to_lowercase(),
                environment: network.environment.to_string(),
                chain_id: if network.chain_id > 0 {
                    Some(network.chain_id)
                } else {
                    None
                },
                strategies: config.strategies.enabled.clone(),
                dry_run: config.dry_run,
            };
            Arc::new(StatusWriter::new(path.clone(), status_config))
        });
        if let Some(ref writer) = status_writer {
            // Write initial status file at startup
            if let Err(e) = writer.write() {
                warn!(error = %e, "Failed to write initial status file");
            }
            info!(path = ?config.status_file, "Status file writer initialized");
        }

        // Initialize executor (optional)
        let executor = init_executor(&config).await;

        // Initialize inference infrastructure
        let cluster_cache = build_cluster_cache(&config);
        let llm_client = build_llm_client(&config);
        let inferrer: Option<Arc<dyn Inferrer>> = llm_client
            .map(|llm| build_inferrer(&config, llm));

        if inferrer.is_some() {
            info!("Inference service enabled");
        }

        // Build strategy registry with cache for combinatorial strategy
        let strategies = Arc::new(build_strategy_registry(
            &config,
            Some(Arc::clone(&cluster_cache)),
        ));
        info!(
            strategies = ?strategies.strategies().iter().map(|s| s.name()).collect::<Vec<_>>(),
            "Strategies loaded"
        );

        // Fetch markets using exchange-agnostic trait
        let market_fetcher = ExchangeFactory::create_market_fetcher(&config);
        info!(
            exchange = market_fetcher.exchange_name(),
            "Fetching markets"
        );
        let market_infos = market_fetcher.get_markets(20).await?;

        if market_infos.is_empty() {
            warn!("No active markets found");
            return Ok(());
        }

        // Parse market info using exchange-specific configuration
        let exchange_config = ExchangeFactory::create_exchange_config(&config);
        let markets = exchange_config.parse_markets(&market_infos);

        // Build generic registry
        let mut registry = MarketRegistry::new();
        for market in markets {
            registry.add(market);
        }

        info!(
            total_markets = market_infos.len(),
            yes_no_pairs = registry.len(),
            "Markets loaded"
        );

        if registry.is_empty() {
            warn!("No YES/NO market pairs found");
            return Ok(());
        }

        for market in registry.markets() {
            debug!(
                market_id = %market.market_id(),
                question = %market.question(),
                "Tracking market"
            );
        }

        // Run initial inference if enabled
        if let Some(ref inf) = inferrer {
            let summaries: Vec<MarketSummary> = registry
                .markets()
                .iter()
                .take(config.inference.batch_size)
                .map(|m| MarketSummary {
                    id: m.market_id().clone(),
                    question: m.question().to_string(),
                    outcomes: m.outcomes().iter().map(|o| o.name().to_string()).collect(),
                })
                .collect();

            if summaries.len() >= 2 {
                info!(markets = summaries.len(), "Running initial inference");
                match inf.infer(&summaries).await {
                    Ok(relations) => {
                        if !relations.is_empty() {
                            info!(
                                relations = relations.len(),
                                "Discovered market relations"
                            );
                            cluster_cache.put_relations(relations);
                        } else {
                            debug!("No relations discovered in initial batch");
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "Initial inference failed");
                    }
                }
            }
        }

        let token_ids: Vec<TokenId> = registry
            .markets()
            .iter()
            .flat_map(|m| m.outcomes().iter().map(|o| o.token_id().clone()))
            .collect();

        info!(tokens = token_ids.len(), "Subscribing to tokens");

        let cache = Arc::new(OrderBookCache::new());
        let registry = Arc::new(registry);

        // Create data stream with reconnection support
        let inner_stream = ExchangeFactory::create_data_stream(&config);
        let mut data_stream =
            ReconnectingDataStream::new(inner_stream, config.reconnection.clone());
        data_stream.connect().await?;
        data_stream.subscribe(&token_ids).await?;

        info!("Listening for market events...");

        let dry_run = config.dry_run;

        // Event loop using trait-based stream
        while let Some(event) = data_stream.next_event().await {
            handle_market_event(
                event,
                &cache,
                &registry,
                &strategies,
                executor.clone(),
                &risk_manager,
                &notifiers,
                &state,
                dry_run,
                status_writer.clone(),
            );
        }

        Ok(())
    }
}
