//! App orchestration module.
//!
//! This module contains the main application logic for running
//! the edgelord arbitrage detection and execution system.

// Allow many arguments for handler functions that coordinate multiple services
#![allow(clippy::too_many_arguments)]

use std::sync::Arc;

use tracing::{debug, error, info, warn};

use crate::core::exchange::polymarket::{Executor as PolymarketExecutor, PolymarketRegistry};
use crate::core::exchange::{ArbitrageExecutionResult, ArbitrageExecutor};
use crate::app::config::Config;
use crate::app::state::AppState;
use crate::app::status_file::{StatusConfig, StatusWriter};
use crate::core::strategy::{
    CombinatorialStrategy, DetectionContext, MarketRebalancingStrategy, SingleConditionStrategy,
    StrategyRegistry,
};
use crate::core::cache::OrderBookCache;
use crate::core::domain::{Opportunity, TokenId};
use crate::error::{Result, RiskError};
use crate::core::exchange::{ExchangeFactory, MarketEvent, OrderId};
use rust_decimal::Decimal;
use crate::core::service::{
    Event, ExecutionEvent, LogNotifier, NotifierRegistry, OpportunityEvent, RiskCheckResult,
    RiskEvent, RiskManager,
};

#[cfg(feature = "telegram")]
use crate::core::service::{TelegramConfig, TelegramNotifier};

/// Main application struct.
pub struct App;

impl App {
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
                chain_id: if network.chain_id > 0 { Some(network.chain_id) } else { None },
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

        // Initialize executor (optional) - still Polymarket-specific for now
        let executor = init_executor(&config).await;

        // Build strategy registry
        let strategies = Arc::new(build_strategy_registry(&config));
        info!(
            strategies = ?strategies.strategies().iter().map(|s| s.name()).collect::<Vec<_>>(),
            "Strategies loaded"
        );

        // Fetch markets using exchange-agnostic trait
        let market_fetcher = ExchangeFactory::create_market_fetcher(&config);
        info!(exchange = market_fetcher.exchange_name(), "Fetching markets");
        let markets = market_fetcher.get_markets(20).await?;

        if markets.is_empty() {
            warn!("No active markets found");
            return Ok(());
        }

        // Build registry from generic MarketInfo
        let registry = PolymarketRegistry::from_market_info(&markets);

        info!(
            total_markets = markets.len(),
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

        let token_ids: Vec<TokenId> = registry
            .markets()
            .iter()
            .flat_map(|m| m.outcomes().iter().map(|o| o.token_id().clone()))
            .collect();

        info!(tokens = token_ids.len(), "Subscribing to tokens");

        let cache = Arc::new(OrderBookCache::new());
        let registry = Arc::new(registry);

        // Create data stream using exchange-agnostic trait
        let mut data_stream = ExchangeFactory::create_data_stream(&config);
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

/// Build notifier registry from configuration.
fn build_notifier_registry(config: &Config) -> NotifierRegistry {
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
fn build_strategy_registry(config: &Config) -> StrategyRegistry {
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
async fn init_executor(config: &Config) -> Option<Arc<PolymarketExecutor>> {
    if config.wallet.private_key.is_some() {
        match PolymarketExecutor::new(config).await {
            Ok(exec) => {
                info!("Executor initialized - trading ENABLED");
                Some(Arc::new(exec))
            }
            Err(e) => {
                warn!(error = %e, "Failed to initialize executor - detection only");
                None
            }
        }
    } else {
        info!("No wallet configured - detection only mode");
        None
    }
}

/// Handle incoming market events from the data stream.
fn handle_market_event(
    event: MarketEvent,
    cache: &OrderBookCache,
    registry: &PolymarketRegistry,
    strategies: &StrategyRegistry,
    executor: Option<Arc<PolymarketExecutor>>,
    risk_manager: &RiskManager,
    notifiers: &Arc<NotifierRegistry>,
    state: &Arc<AppState>,
    dry_run: bool,
    status_writer: Option<Arc<StatusWriter>>,
) {
    match event {
        MarketEvent::OrderBookSnapshot { token_id, book } => {
            cache.update(book);

            if let Some(market) = registry.get_market_for_token(&token_id) {
                let ctx = DetectionContext::new(market, cache);
                let opportunities = strategies.detect_all(&ctx);

                for opp in opportunities {
                    handle_opportunity(opp, executor.clone(), risk_manager, notifiers, state, cache, dry_run, status_writer.clone());
                }
            }
        }
        MarketEvent::OrderBookDelta { token_id, book } => {
            // For now, treat deltas as snapshots (simple approach)
            cache.update(book);

            if let Some(market) = registry.get_market_for_token(&token_id) {
                let ctx = DetectionContext::new(market, cache);
                let opportunities = strategies.detect_all(&ctx);

                for opp in opportunities {
                    handle_opportunity(opp, executor.clone(), risk_manager, notifiers, state, cache, dry_run, status_writer.clone());
                }
            }
        }
        MarketEvent::Connected => {
            info!("Data stream connected");
        }
        MarketEvent::Disconnected { reason } => {
            warn!(reason = %reason, "Data stream disconnected");
        }
    }
}

/// Handle a detected opportunity.
fn handle_opportunity(
    opp: Opportunity,
    executor: Option<Arc<PolymarketExecutor>>,
    risk_manager: &RiskManager,
    notifiers: &Arc<NotifierRegistry>,
    state: &Arc<AppState>,
    cache: &OrderBookCache,
    dry_run: bool,
    status_writer: Option<Arc<StatusWriter>>,
) {
    // Check for duplicate execution
    if !state.try_lock_execution(opp.market_id().as_str()) {
        debug!(market_id = %opp.market_id(), "Execution already in progress, skipping");
        return;
    }

    // Pre-execution slippage check
    let max_slippage = state.risk_limits().max_slippage;
    if let Some(slippage) = get_max_slippage(&opp, cache) {
        if slippage > max_slippage {
            debug!(
                market_id = %opp.market_id(),
                slippage = %slippage,
                max = %max_slippage,
                "Slippage check failed, rejecting opportunity"
            );
            state.release_execution(opp.market_id().as_str());
            let error = RiskError::SlippageTooHigh {
                actual: slippage,
                max: max_slippage,
            };
            notifiers.notify_all(Event::RiskRejected(RiskEvent::new(
                opp.market_id().as_str(),
                &error,
            )));
            return;
        }
    }

    // Record opportunity in status file
    if let Some(ref writer) = status_writer {
        writer.record_opportunity();
        if let Err(e) = writer.write() {
            warn!(error = %e, "Failed to write status file");
        }
    }

    // Notify opportunity detected
    notifiers.notify_all(Event::OpportunityDetected(OpportunityEvent::from(&opp)));

    // Check risk
    match risk_manager.check(&opp) {
        RiskCheckResult::Approved => {
            if dry_run {
                info!(
                    market_id = %opp.market_id(),
                    edge = %opp.edge(),
                    profit = %opp.expected_profit(),
                    "Dry-run: would execute trade"
                );
                state.release_execution(opp.market_id().as_str());
            } else if let Some(exec) = executor {
                spawn_execution(exec, opp, notifiers.clone(), state.clone(), status_writer);
            } else {
                // No executor, release the lock
                state.release_execution(opp.market_id().as_str());
            }
        }
        RiskCheckResult::Rejected(error) => {
            // Release the lock on rejection
            state.release_execution(opp.market_id().as_str());
            notifiers.notify_all(Event::RiskRejected(RiskEvent::new(
                opp.market_id().as_str(),
                &error,
            )));
        }
    }
}

/// Get the maximum slippage across all legs.
/// Returns None if prices cannot be determined (books not in cache or empty).
fn get_max_slippage(opportunity: &Opportunity, cache: &OrderBookCache) -> Option<Decimal> {
    let mut max_slippage = Decimal::ZERO;

    for leg in opportunity.legs() {
        let book = cache.get(leg.token_id())?;
        let current_price = book.best_ask()?.price();
        let expected_price = leg.ask_price();

        if expected_price == Decimal::ZERO {
            return None;
        }

        let slippage = ((current_price - expected_price).abs()) / expected_price;
        max_slippage = max_slippage.max(slippage);
    }

    Some(max_slippage)
}

/// Spawn async execution without blocking message processing.
fn spawn_execution(
    executor: Arc<PolymarketExecutor>,
    opportunity: Opportunity,
    notifiers: Arc<NotifierRegistry>,
    state: Arc<AppState>,
    status_writer: Option<Arc<StatusWriter>>,
) {
    let market_id = opportunity.market_id().to_string();
    let expected_profit = opportunity.expected_profit();

    tokio::spawn(async move {
        let result = executor.execute_arbitrage(&opportunity).await;

        // Always release the execution lock
        state.release_execution(&market_id);

        match result {
            Ok(exec_result) => {
                match &exec_result {
                    ArbitrageExecutionResult::Success { .. } => {
                        record_position(&state, &opportunity);
                        // Record execution with profit in status file
                        if let Some(ref writer) = status_writer {
                            writer.record_execution(expected_profit);
                            // Update runtime stats
                            let positions = state.positions();
                            let open_count = positions.open_positions().count();
                            let exposure = positions.total_exposure();
                            let max_exposure = state.risk_limits().max_total_exposure;
                            writer.update_runtime(open_count, exposure, max_exposure);
                            if let Err(e) = writer.write() {
                                warn!(error = %e, "Failed to write status file");
                            }
                        }
                    }
                    ArbitrageExecutionResult::PartialFill { filled, failed } => {
                        let filled_ids: Vec<_> = filled.iter().map(|f| f.token_id.to_string()).collect();
                        let failed_ids: Vec<_> = failed.iter().map(|f| f.token_id.to_string()).collect();
                        warn!(
                            filled = ?filled_ids,
                            failed = ?failed_ids,
                            "Partial fill detected, attempting recovery"
                        );

                        // Try to cancel all filled orders
                        let mut cancel_failed = false;
                        for fill in filled {
                            let order_id = OrderId::new(fill.order_id.clone());
                            if let Err(cancel_err) = ArbitrageExecutor::cancel(executor.as_ref(), &order_id).await {
                                warn!(error = %cancel_err, token = %fill.token_id, "Failed to cancel filled leg");
                                cancel_failed = true;
                            }
                        }

                        if cancel_failed {
                            warn!("Some cancellations failed, recording partial position");
                            record_partial_position(&state, &opportunity, filled, failed);
                            if let Some(ref writer) = status_writer {
                                let positions = state.positions();
                                let open_count = positions.open_positions().count();
                                let exposure = positions.total_exposure();
                                let max_exposure = state.risk_limits().max_total_exposure;
                                writer.update_runtime(open_count, exposure, max_exposure);
                                if let Err(e) = writer.write() {
                                    warn!(error = %e, "Failed to write status file");
                                }
                            }
                        } else {
                            info!("Successfully cancelled all filled legs, no position recorded");
                        }
                    }
                    ArbitrageExecutionResult::Failed { .. } => {}
                }

                // Notify execution result
                notifiers.notify_all(Event::ExecutionCompleted(ExecutionEvent::from_result(
                    &market_id, &exec_result,
                )));
            }
            Err(e) => {
                error!(error = %e, "Execution failed");
                notifiers.notify_all(Event::ExecutionCompleted(ExecutionEvent {
                    market_id,
                    success: false,
                    details: e.to_string(),
                }));
            }
        }
    });
}

/// Record a position in shared state.
fn record_position(state: &AppState, opportunity: &Opportunity) {
    use crate::core::domain::{Position, PositionLeg, PositionStatus};

    let position_legs: Vec<PositionLeg> = opportunity
        .legs()
        .iter()
        .map(|leg| PositionLeg::new(leg.token_id().clone(), opportunity.volume(), leg.ask_price()))
        .collect();

    let mut positions = state.positions_mut();
    let position = Position::new(
        positions.next_id(),
        opportunity.market_id().clone(),
        position_legs,
        opportunity.total_cost() * opportunity.volume(),
        opportunity.volume(),
        chrono::Utc::now(),
        PositionStatus::Open,
    );
    positions.add(position);
}

/// Record a partial fill position.
fn record_partial_position(
    state: &AppState,
    opportunity: &Opportunity,
    filled: &[crate::core::exchange::FilledLeg],
    failed: &[crate::core::exchange::FailedLeg],
) {
    use crate::core::domain::{Position, PositionLeg, PositionStatus};

    let filled_token_ids: Vec<TokenId> = filled.iter().map(|f| f.token_id.clone()).collect();
    let missing_token_ids: Vec<TokenId> = failed.iter().map(|f| f.token_id.clone()).collect();

    // Build position legs from filled legs
    let position_legs: Vec<PositionLeg> = opportunity
        .legs()
        .iter()
        .filter(|leg| filled_token_ids.contains(leg.token_id()))
        .map(|leg| PositionLeg::new(leg.token_id().clone(), opportunity.volume(), leg.ask_price()))
        .collect();

    let entry_cost: Decimal = position_legs.iter().map(|l| l.entry_price() * l.size()).sum();

    let mut positions = state.positions_mut();
    let position = Position::new(
        positions.next_id(),
        opportunity.market_id().clone(),
        position_legs,
        entry_cost,
        opportunity.volume(),
        chrono::Utc::now(),
        PositionStatus::PartialFill {
            filled: filled_token_ids,
            missing: missing_token_ids,
        },
    );
    positions.add(position);
}
