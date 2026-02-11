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
        #[allow(unused_variables)]
        let (notifiers, runtime_stats) = build_notifier_registry(
            &config,
            Arc::clone(&state),
            Arc::clone(&stats_recorder),
        );
        let notifiers = Arc::new(notifiers);
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

        // Update runtime stats for Telegram commands
        #[cfg(feature = "telegram")]
        if let Some(ref stats) = runtime_stats {
            stats.update_market_counts(registry.len(), token_ids.len());
        }

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

        // Timer for periodic stats updates (configurable, default 30 seconds)
        let stats_interval_secs = config.telegram.stats_interval_secs;
        let mut stats_interval = tokio::time::interval(std::time::Duration::from_secs(stats_interval_secs));
        stats_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

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
                _ = stats_interval.tick() => {
                    // Update pool stats for Telegram commands
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app::Config;

    #[test]
    fn health_check_struct_accessors() {
        let check = HealthCheck {
            name: "test_service",
            critical: true,
            status: HealthStatus::Healthy,
        };

        assert_eq!(check.name(), "test_service");
        assert!(check.critical());
        assert!(matches!(check.status(), HealthStatus::Healthy));
        assert!(check.is_healthy());
    }

    #[test]
    fn health_check_unhealthy_status() {
        let check = HealthCheck {
            name: "broken_service",
            critical: false,
            status: HealthStatus::Unhealthy("connection failed".to_string()),
        };

        assert!(!check.is_healthy());
        assert!(matches!(check.status(), HealthStatus::Unhealthy(_)));
    }

    #[test]
    fn health_report_is_healthy_when_all_critical_pass() {
        let report = HealthReport {
            checks: vec![
                HealthCheck {
                    name: "critical_pass",
                    critical: true,
                    status: HealthStatus::Healthy,
                },
                HealthCheck {
                    name: "non_critical_fail",
                    critical: false,
                    status: HealthStatus::Unhealthy("warning".to_string()),
                },
            ],
        };

        assert!(report.is_healthy());
    }

    #[test]
    fn health_report_is_unhealthy_when_critical_fails() {
        let report = HealthReport {
            checks: vec![
                HealthCheck {
                    name: "critical_fail",
                    critical: true,
                    status: HealthStatus::Unhealthy("error".to_string()),
                },
                HealthCheck {
                    name: "critical_pass",
                    critical: true,
                    status: HealthStatus::Healthy,
                },
            ],
        };

        assert!(!report.is_healthy());
    }

    #[test]
    fn health_report_checks_accessor() {
        let report = HealthReport {
            checks: vec![
                HealthCheck {
                    name: "check1",
                    critical: true,
                    status: HealthStatus::Healthy,
                },
                HealthCheck {
                    name: "check2",
                    critical: false,
                    status: HealthStatus::Healthy,
                },
            ],
        };

        assert_eq!(report.checks().len(), 2);
    }

    #[test]
    fn health_check_with_default_config() {
        let config = Config::default();
        let report = health_check(&config);

        assert!(report.checks().len() >= 4);

        let check_names: Vec<_> = report.checks().iter().map(|c| c.name()).collect();
        assert!(check_names.contains(&"database"));
        assert!(check_names.contains(&"exchange_api"));
        assert!(check_names.contains(&"exchange_ws"));
        assert!(check_names.contains(&"strategies"));
    }

    #[test]
    fn health_check_detects_empty_database_path() {
        let config = Config {
            database: String::new(),
            ..Default::default()
        };

        let report = health_check(&config);
        let db_check = report
            .checks()
            .iter()
            .find(|c| c.name() == "database")
            .unwrap();

        assert!(!db_check.is_healthy());
    }

    #[test]
    fn health_check_detects_empty_api_url() {
        let mut config = Config::default();
        match &mut config.exchange_config {
            crate::app::ExchangeSpecificConfig::Polymarket(pm) => {
                pm.api_url = String::new();
            }
        }

        let report = health_check(&config);
        let api_check = report
            .checks()
            .iter()
            .find(|c| c.name() == "exchange_api")
            .unwrap();

        assert!(!api_check.is_healthy());
    }

    #[test]
    fn health_check_detects_empty_ws_url() {
        let mut config = Config::default();
        match &mut config.exchange_config {
            crate::app::ExchangeSpecificConfig::Polymarket(pm) => {
                pm.ws_url = String::new();
            }
        }

        let report = health_check(&config);
        let ws_check = report
            .checks()
            .iter()
            .find(|c| c.name() == "exchange_ws")
            .unwrap();

        assert!(!ws_check.is_healthy());
    }

    #[test]
    fn health_check_detects_no_strategies_enabled() {
        let mut config = Config::default();
        config.strategies.enabled.clear();

        let report = health_check(&config);
        let strat_check = report
            .checks()
            .iter()
            .find(|c| c.name() == "strategies")
            .unwrap();

        assert!(!strat_check.is_healthy());
    }

    #[test]
    fn health_status_equality() {
        assert_eq!(HealthStatus::Healthy, HealthStatus::Healthy);
        assert_eq!(
            HealthStatus::Unhealthy("a".to_string()),
            HealthStatus::Unhealthy("a".to_string())
        );
        assert_ne!(
            HealthStatus::Healthy,
            HealthStatus::Unhealthy("error".to_string())
        );
    }

    #[test]
    fn event_processing_context_can_be_created() {
        use crate::app::AppState;
        use crate::core::cache::OrderBookCache;
        use crate::core::domain::MarketRegistry;
        use crate::core::service::{NotifierRegistry, RiskManager};
        use crate::core::strategy::StrategyRegistry;
        use std::sync::Arc;

        let cache = OrderBookCache::new();
        let registry = MarketRegistry::new();
        let strategies = StrategyRegistry::new();
        let state = Arc::new(AppState::default());
        let notifiers = Arc::new(NotifierRegistry::new());
        let risk_manager = RiskManager::new(Arc::clone(&state));
        let db_pool = crate::core::db::create_pool("sqlite://:memory:").unwrap();
        let stats = crate::core::service::statistics::create_recorder(db_pool);
        let position_manager = Arc::new(crate::core::service::position::PositionManager::new(
            Arc::clone(&stats),
        ));

        let _ctx = EventProcessingContext {
            cache: &cache,
            registry: &registry,
            strategies: &strategies,
            executor: None,
            risk_manager: &risk_manager,
            notifiers: &notifiers,
            state: &state,
            stats: &stats,
            position_manager: &position_manager,
            dry_run: true,
        };
    }
}
