//! App orchestration module.
//!
//! This module contains the main application logic for running
//! the edgelord arbitrage detection and execution system.

// Allow many arguments for handler functions that coordinate multiple services
#![allow(clippy::too_many_arguments)]

use std::sync::Arc;

use tracing::{debug, error, info, warn};

use crate::adapter::polymarket::{
    ArbitrageExecutionResult, Client as PolymarketClient, Executor as PolymarketExecutor,
    MarketRegistry, WebSocketHandler, WsMessage,
};
use crate::app::config::Config;
use crate::app::state::AppState;
use crate::domain::strategy::{
    CombinatorialStrategy, DetectionContext, MarketRebalancingStrategy, SingleConditionStrategy,
    StrategyRegistry,
};
use crate::domain::{Opportunity, OrderBookCache};
use crate::error::Result;
use crate::service::{
    Event, ExecutionEvent, LogNotifier, NotifierRegistry, OpportunityEvent, RiskCheckResult,
    RiskEvent, RiskManager,
};

#[cfg(feature = "telegram")]
use crate::service::{TelegramConfig, TelegramNotifier};

/// Main application struct.
pub struct App;

impl App {
    /// Run the main application loop.
    pub async fn run(config: Config) -> Result<()> {
        // Initialize shared state
        let state = Arc::new(AppState::new(config.risk.clone().into()));

        // Initialize risk manager
        let risk_manager = Arc::new(RiskManager::new(state.clone()));

        // Initialize notifiers
        let notifiers = Arc::new(build_notifier_registry(&config));
        info!(notifiers = notifiers.len(), "Notifiers initialized");

        // Initialize executor (optional)
        let executor = init_executor(&config).await;

        // Build strategy registry
        let strategies = Arc::new(build_strategy_registry(&config));
        info!(
            strategies = ?strategies.strategies().iter().map(|s| s.name()).collect::<Vec<_>>(),
            "Strategies loaded"
        );

        // Fetch markets
        let client = PolymarketClient::new(config.network.api_url.clone());
        let markets = client.get_active_markets(20).await?;

        if markets.is_empty() {
            warn!("No active markets found");
            return Ok(());
        }

        let registry = MarketRegistry::from_markets(&markets);

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

        let token_ids: Vec<String> = registry
            .pairs()
            .iter()
            .flat_map(|p| vec![p.yes_token().to_string(), p.no_token().to_string()])
            .collect();

        info!(tokens = token_ids.len(), "Subscribing to tokens");

        let cache = Arc::new(OrderBookCache::new());
        let registry = Arc::new(registry);

        let handler = WebSocketHandler::new(config.network.ws_url);

        // Clone Arcs for closure
        let cache_clone = cache.clone();
        let registry_clone = registry.clone();
        let strategies_clone = strategies.clone();
        let executor_clone = executor.clone();
        let risk_manager_clone = risk_manager.clone();
        let notifiers_clone = notifiers.clone();
        let state_clone = state.clone();

        handler
            .run(token_ids, move |msg| {
                handle_message(
                    msg,
                    &cache_clone,
                    &registry_clone,
                    &strategies_clone,
                    executor_clone.clone(),
                    &risk_manager_clone,
                    &notifiers_clone,
                    &state_clone,
                );
            })
            .await?;

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

/// Handle incoming WebSocket messages.
fn handle_message(
    msg: WsMessage,
    cache: &OrderBookCache,
    registry: &MarketRegistry,
    strategies: &StrategyRegistry,
    executor: Option<Arc<PolymarketExecutor>>,
    risk_manager: &RiskManager,
    notifiers: &Arc<NotifierRegistry>,
    state: &Arc<AppState>,
) {
    match msg {
        WsMessage::Book(book) => {
            let orderbook = book.to_orderbook();
            let token_id = orderbook.token_id().clone();
            cache.update(orderbook);

            if let Some(pair) = registry.get_market_for_token(&token_id) {
                let ctx = DetectionContext::new(pair, cache);
                let opportunities = strategies.detect_all(&ctx);

                for opp in opportunities {
                    handle_opportunity(opp, executor.clone(), risk_manager, notifiers, state);
                }
            }
        }
            // Price changes are incremental updates; we only need full book snapshots
            // for arbitrage detection since we need both bid and ask sides
            WsMessage::PriceChange(_) => {}
        _ => {}
    }
}

/// Handle a detected opportunity.
fn handle_opportunity(
    opp: Opportunity,
    executor: Option<Arc<PolymarketExecutor>>,
    risk_manager: &RiskManager,
    notifiers: &Arc<NotifierRegistry>,
    state: &Arc<AppState>,
) {
    // Notify opportunity detected
    notifiers.notify_all(Event::OpportunityDetected(OpportunityEvent::from(&opp)));

    // Check risk
    match risk_manager.check(&opp) {
        RiskCheckResult::Approved => {
            if let Some(exec) = executor {
                spawn_execution(exec, opp, notifiers.clone(), state.clone());
            }
        }
        RiskCheckResult::Rejected(error) => {
            notifiers.notify_all(Event::RiskRejected(RiskEvent::new(
                opp.market_id().as_str(),
                &error,
            )));
        }
    }
}

/// Spawn async execution without blocking message processing.
fn spawn_execution(
    executor: Arc<PolymarketExecutor>,
    opportunity: Opportunity,
    notifiers: Arc<NotifierRegistry>,
    state: Arc<AppState>,
) {
    let market_id = opportunity.market_id().to_string();

    tokio::spawn(async move {
        match executor.execute_arbitrage(&opportunity).await {
            Ok(result) => {
                // Record position in shared state
                if matches!(result, ArbitrageExecutionResult::Success { .. }) {
                    record_position(&state, &opportunity);
                }

                // Notify execution result
                notifiers.notify_all(Event::ExecutionCompleted(ExecutionEvent::from_result(
                    &market_id, &result,
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
    use crate::domain::{Position, PositionLeg, PositionStatus};

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
