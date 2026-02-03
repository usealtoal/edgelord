//! App orchestration module.
//!
//! This module contains the main application logic for running
//! the edgelord arbitrage detection and execution system.

// Allow many arguments for handler functions that coordinate multiple services
#![allow(clippy::too_many_arguments)]

use std::sync::Arc;

use tracing::{debug, error, info, warn};

use crate::core::exchange::polymarket::{ArbitrageExecutionResult, Executor as PolymarketExecutor, MarketRegistry};
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
use crate::core::exchange::{ExchangeFactory, MarketEvent, OrderExecutor, OrderId};
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
            let status_config = StatusConfig {
                chain_id: config.network.chain_id,
                network: if config.network.chain_id == 137 {
                    "mainnet".to_string()
                } else {
                    "testnet".to_string()
                },
                strategies: config.strategies.enabled.clone(),
                dry_run: config.dry_run,
            };
            Arc::new(StatusWriter::new(path.clone(), status_config))
        });
        if status_writer.is_some() {
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
        let registry = MarketRegistry::from_market_info(&markets);

        info!(
            total_markets = markets.len(),
            yes_no_pairs = registry.len(),
            "Markets loaded"
        );

        if registry.is_empty() {
            warn!("No YES/NO market pairs found");
            return Ok(());
        }

        for pair in registry.pairs() {
            debug!(
                market_id = %pair.market_id(),
                question = %pair.question(),
                "Tracking market"
            );
        }

        let token_ids: Vec<TokenId> = registry
            .pairs()
            .iter()
            .flat_map(|p| vec![p.yes_token().clone(), p.no_token().clone()])
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
    registry: &MarketRegistry,
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

            if let Some(pair) = registry.get_market_for_token(&token_id) {
                let ctx = DetectionContext::new(pair, cache);
                let opportunities = strategies.detect_all(&ctx);

                for opp in opportunities {
                    handle_opportunity(opp, executor.clone(), risk_manager, notifiers, state, cache, dry_run, status_writer.clone());
                }
            }
        }
        MarketEvent::OrderBookDelta { token_id, book } => {
            // For now, treat deltas as snapshots (simple approach)
            cache.update(book);

            if let Some(pair) = registry.get_market_for_token(&token_id) {
                let ctx = DetectionContext::new(pair, cache);
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

/// Get the maximum slippage across both legs.
/// Returns None if prices cannot be determined (books not in cache or empty).
fn get_max_slippage(opportunity: &Opportunity, cache: &OrderBookCache) -> Option<Decimal> {
    let yes_book = cache.get(opportunity.yes_token())?;
    let no_book = cache.get(opportunity.no_token())?;

    let yes_current = yes_book.best_ask()?.price();
    let no_current = no_book.best_ask()?.price();

    let yes_expected = opportunity.yes_ask();
    let no_expected = opportunity.no_ask();

    // Avoid division by zero
    if yes_expected == Decimal::ZERO || no_expected == Decimal::ZERO {
        return None;
    }

    let yes_slippage = ((yes_current - yes_expected).abs()) / yes_expected;
    let no_slippage = ((no_current - no_expected).abs()) / no_expected;

    Some(yes_slippage.max(no_slippage))
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
                        }
                    }
                    ArbitrageExecutionResult::PartialFill {
                        filled_leg,
                        filled_order_id,
                        failed_leg,
                        error: err_msg,
                    } => {
                        warn!(
                            filled_leg = %filled_leg,
                            failed_leg = %failed_leg,
                            error = %err_msg,
                            "Partial fill detected, attempting recovery"
                        );

                        // Try to cancel the filled order
                        let order_id = OrderId::new(filled_order_id.clone());
                        if let Err(cancel_err) = executor.cancel(&order_id).await {
                            warn!(error = %cancel_err, "Failed to cancel filled leg, recording partial position");
                            record_partial_position(&state, &opportunity, filled_leg);
                            // Update runtime stats for partial position
                            if let Some(ref writer) = status_writer {
                                let positions = state.positions();
                                let open_count = positions.open_positions().count();
                                let exposure = positions.total_exposure();
                                let max_exposure = state.risk_limits().max_total_exposure;
                                writer.update_runtime(open_count, exposure, max_exposure);
                            }
                        } else {
                            info!("Successfully cancelled filled leg, no position recorded");
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

    let mut positions = state.positions_mut();
    let position = Position::new(
        positions.next_id(),
        opportunity.market_id().clone(),
        vec![
            PositionLeg::new(
                opportunity.yes_token().clone(),
                opportunity.volume(),
                opportunity.yes_ask(),
            ),
            PositionLeg::new(
                opportunity.no_token().clone(),
                opportunity.volume(),
                opportunity.no_ask(),
            ),
        ],
        opportunity.total_cost() * opportunity.volume(),
        opportunity.volume(),
        chrono::Utc::now(),
        PositionStatus::Open,
    );
    positions.add(position);
}

/// Record a partial fill position (only one leg filled).
fn record_partial_position(state: &AppState, opportunity: &Opportunity, filled_leg: &TokenId) {
    use crate::core::domain::{Position, PositionLeg, PositionStatus};

    let (token, price, missing) = if filled_leg == opportunity.yes_token() {
        (
            opportunity.yes_token().clone(),
            opportunity.yes_ask(),
            opportunity.no_token().clone(),
        )
    } else {
        (
            opportunity.no_token().clone(),
            opportunity.no_ask(),
            opportunity.yes_token().clone(),
        )
    };

    let mut positions = state.positions_mut();
    let position = Position::new(
        positions.next_id(),
        opportunity.market_id().clone(),
        vec![PositionLeg::new(token.clone(), opportunity.volume(), price)],
        price * opportunity.volume(),
        opportunity.volume(),
        chrono::Utc::now(),
        PositionStatus::PartialFill {
            filled: vec![token],
            missing: vec![missing],
        },
    );
    positions.add(position);
}
