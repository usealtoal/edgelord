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

use tokio::sync::watch;
use tracing::{debug, info, warn};

use crate::app::config::Config;
use crate::app::state::AppState;
use crate::core::cache::OrderBookCache;
use crate::core::db;
use crate::core::domain::{MarketRegistry, TokenId};
use crate::core::exchange::{
    ArbitrageExecutor, ExchangeFactory, MarketDataStream, MarketEvent, ReconnectingDataStream,
};
use crate::core::inference::{Inferrer, MarketSummary};
use crate::core::service::cluster::ClusterDetectionService;
use crate::core::service::position::PositionManager;
use crate::core::service::statistics;
use crate::core::service::{Event, NotifierRegistry, OpportunityEvent, RiskManager, StatsRecorder};
use crate::core::strategy::StrategyRegistry;
use crate::error::Result;

use builder::{
    build_cluster_cache, build_inferrer, build_llm_client, build_notifier_registry,
    build_strategy_registry, init_executor,
};
use handler::handle_market_event;

/// Inputs required to process one market event through detection and risk gates.
pub struct EventProcessingContext<'a> {
    pub cache: &'a OrderBookCache,
    pub registry: &'a MarketRegistry,
    pub strategies: &'a StrategyRegistry,
    pub executor: Option<Arc<dyn ArbitrageExecutor + Send + Sync>>,
    pub risk_manager: &'a RiskManager,
    pub notifiers: &'a Arc<NotifierRegistry>,
    pub state: &'a Arc<AppState>,
    pub stats: &'a Arc<StatsRecorder>,
    pub position_manager: &'a Arc<PositionManager>,
    pub dry_run: bool,
}

/// Process a single market event through the orchestrator pipeline.
pub fn process_market_event(event: MarketEvent, context: EventProcessingContext<'_>) {
    handle_market_event(
        event,
        context.cache,
        context.registry,
        context.strategies,
        context.executor,
        context.risk_manager,
        context.notifiers,
        context.state,
        context.stats,
        context.position_manager,
        context.dry_run,
    );
}

/// Main application orchestrator.
pub(crate) struct Orchestrator;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Unhealthy(String),
}

#[derive(Debug, Clone)]
pub struct HealthCheck {
    name: &'static str,
    critical: bool,
    status: HealthStatus,
}

impl HealthCheck {
    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn critical(&self) -> bool {
        self.critical
    }

    pub fn status(&self) -> &HealthStatus {
        &self.status
    }

    pub fn is_healthy(&self) -> bool {
        matches!(self.status, HealthStatus::Healthy)
    }
}

#[derive(Debug, Clone)]
pub struct HealthReport {
    checks: Vec<HealthCheck>,
}

impl HealthReport {
    pub fn checks(&self) -> &[HealthCheck] {
        &self.checks
    }

    pub fn is_healthy(&self) -> bool {
        self.checks
            .iter()
            .filter(|check| check.critical())
            .all(HealthCheck::is_healthy)
    }
}

pub fn health_check(config: &Config) -> HealthReport {
    let network = config.network();
    let mut checks = Vec::new();

    checks.push(HealthCheck {
        name: "database",
        critical: true,
        status: if config.database.trim().is_empty() {
            HealthStatus::Unhealthy("database path is empty".to_string())
        } else {
            HealthStatus::Healthy
        },
    });

    checks.push(HealthCheck {
        name: "exchange_api",
        critical: true,
        status: if network.api_url.trim().is_empty() {
            HealthStatus::Unhealthy("api_url is empty".to_string())
        } else {
            HealthStatus::Healthy
        },
    });

    checks.push(HealthCheck {
        name: "exchange_ws",
        critical: true,
        status: if network.ws_url.trim().is_empty() {
            HealthStatus::Unhealthy("ws_url is empty".to_string())
        } else {
            HealthStatus::Healthy
        },
    });

    checks.push(HealthCheck {
        name: "strategies",
        critical: true,
        status: if config.strategies.enabled.is_empty() {
            HealthStatus::Unhealthy("no strategies enabled".to_string())
        } else {
            HealthStatus::Healthy
        },
    });

    HealthReport { checks }
}

impl Orchestrator {
    /// Run the main application loop.
    pub async fn run(config: Config) -> Result<()> {
        let (_shutdown_tx, shutdown_rx) = watch::channel(false);
        Self::run_with_shutdown(config, shutdown_rx).await
    }

    pub async fn run_with_shutdown(
        config: Config,
        mut shutdown: watch::Receiver<bool>,
    ) -> Result<()> {
        info!(exchange = ?config.exchange, dry_run = config.dry_run, "Starting edgelord");

        // Initialize shared state
        let state = Arc::new(AppState::new(config.risk.clone().into()));

        // Initialize database and run migrations
        let db_url = format!("sqlite://{}", config.database);
        let db_pool = db::create_pool(&db_url)?;
        db::run_migrations(&db_pool)?;
        let stats_recorder = statistics::create_recorder(db_pool);
        let position_manager = Arc::new(PositionManager::new(Arc::clone(&stats_recorder)));
        info!(database = %config.database, "Database initialized");

        // Initialize risk manager
        let risk_manager = Arc::new(RiskManager::new(state.clone()));

        // Initialize notifiers
        let notifiers = Arc::new(build_notifier_registry(&config, Arc::clone(&state)));
        info!(notifiers = notifiers.len(), "Notifiers initialized");

        // Initialize executor (optional)
        let executor = init_executor(&config).await;

        // Initialize inference infrastructure
        let cluster_cache = build_cluster_cache(&config);
        let llm_client = build_llm_client(&config);
        let inferrer: Option<Arc<dyn Inferrer>> =
            llm_client.map(|llm| build_inferrer(&config, llm));

        if inferrer.is_some() {
            info!("Inference service enabled");
        }

        // Build strategy registry with cache for combinatorial strategy
        let strategies = Arc::new(build_strategy_registry(&config, Arc::clone(&cluster_cache)));
        info!(
            strategies = ?strategies.strategies().iter().map(|s| s.name()).collect::<Vec<_>>(),
            "Strategies loaded"
        );

        // Extract max_markets from exchange config
        let max_markets = match &config.exchange_config {
            crate::app::ExchangeSpecificConfig::Polymarket(pm_config) => {
                pm_config.market_filter.max_markets
            }
        };

        // Fetch markets using exchange-agnostic trait
        let market_fetcher = ExchangeFactory::create_market_fetcher(&config);
        info!(
            exchange = market_fetcher.exchange_name(),
            max_markets,
            "Fetching markets"
        );
        let market_infos = market_fetcher.get_markets(max_markets).await?;
        let markets_fetched = market_infos.len();

        if market_infos.is_empty() {
            warn!("No active markets found");
            return Ok(());
        }

        // Apply volume/liquidity filter before subscribing
        let market_filter = ExchangeFactory::create_filter(&config)?;
        let market_infos = market_filter.filter(&market_infos);
        let markets_filtered = market_infos.len();

        info!(
            markets_fetched,
            markets_filtered,
            rejected = markets_fetched - markets_filtered,
            "Volume/liquidity filter applied"
        );

        if market_infos.is_empty() {
            warn!("No markets passed volume/liquidity filter");
            return Ok(());
        }

        // Parse market info using exchange-specific configuration
        let exchange_config = ExchangeFactory::create_exchange_config(&config);
        let markets = exchange_config.parse_markets(&market_infos);
        let markets_parsed = markets.len();

        // Build generic registry
        let mut registry = MarketRegistry::new();
        for market in markets {
            registry.add(market);
        }

        info!(
            markets_fetched,
            markets_parsed,
            yes_no_pairs = registry.len(),
            "Market scan complete"
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
                            info!(relations = relations.len(), "Discovered market relations");
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

        let registry = Arc::new(registry);

        // Create order book cache (with notifications if cluster detection enabled)
        let (cache, cluster_handle) = if config.cluster_detection.enabled {
            let (cache, update_rx) =
                OrderBookCache::with_notifications(config.cluster_detection.channel_capacity);
            let cache = Arc::new(cache);

            // Start cluster detection service
            let service = ClusterDetectionService::new(
                config.cluster_detection.to_core_config(),
                Arc::clone(&cache),
                Arc::clone(&cluster_cache),
                Arc::clone(&registry),
            );
            let (handle, mut opp_rx) = service.start(update_rx);

            // Spawn task to handle cluster opportunities
            let notifiers_clone = Arc::clone(&notifiers);
            tokio::spawn(async move {
                while let Some(opp) = opp_rx.recv().await {
                    info!(
                        cluster = %opp.cluster_id,
                        gap = %opp.gap,
                        markets = ?opp.markets.iter().map(|m| m.as_str()).collect::<Vec<_>>(),
                        "Cluster opportunity detected"
                    );
                    // Notify about the opportunity
                    let event =
                        Event::OpportunityDetected(OpportunityEvent::from(&opp.opportunity));
                    notifiers_clone.notify_all(event);
                }
            });

            info!("Cluster detection service started");
            (cache, Some(handle))
        } else {
            (Arc::new(OrderBookCache::new()), None)
        };

        let cluster_handle = cluster_handle;

        // Create data stream with optional connection pooling
        let mut data_stream: Box<dyn MarketDataStream> =
            if let Some(pool) = ExchangeFactory::create_connection_pool(&config)? {
                info!(
                    exchange = pool.exchange_name(),
                    "Using connection pool"
                );
                Box::new(pool)
            } else {
                info!("Using single connection");
                let inner = ExchangeFactory::create_data_stream(&config);
                Box::new(ReconnectingDataStream::new(
                    inner,
                    config.reconnection.clone(),
                ))
            };

        data_stream.connect().await?;
        data_stream.subscribe(&token_ids).await?;

        info!("Listening for market events...");

        let dry_run = config.dry_run;

        // Event loop using trait-based stream
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
                event = data_stream.next_event() => {
                    let Some(event) = event else {
                        warn!("Market data stream ended");
                        break;
                    };
                    handle_market_event(
                        event,
                        &cache,
                        &registry,
                        &strategies,
                        executor.clone(),
                        &risk_manager,
                        &notifiers,
                        &state,
                        &stats_recorder,
                        &position_manager,
                        dry_run,
                    );
                }
            }
        }

        if let Some(handle) = cluster_handle {
            handle.shutdown().await;
        }

        Ok(())
    }
}
