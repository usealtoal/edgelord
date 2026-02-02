//! App orchestration module.
//!
//! This module contains the main application logic for running
//! the edgelord arbitrage detection and execution system.
//!
//! Requires the `polymarket` feature.

use std::sync::Arc;

use crate::config::Config;
use crate::domain::strategy::{
    CombinatorialStrategy, DetectionContext, MarketRebalancingStrategy, SingleConditionStrategy,
    StrategyRegistry,
};
use crate::domain::{MarketPair, Opportunity, OrderBookCache};
use crate::error::Result;
use crate::polymarket::{
    MarketRegistry, PolymarketClient, PolymarketExecutor, WebSocketHandler, WsMessage,
};
use tracing::{debug, error, info, warn};

/// Main application struct.
pub struct App;

impl App {
    /// Run the main application loop.
    ///
    /// This initializes the executor (if wallet is configured), fetches markets,
    /// and starts the WebSocket handler for real-time order book updates.
    pub async fn run(config: Config) -> Result<()> {
        let executor = init_executor(&config).await;

        // Build strategy registry from config
        let strategies = build_strategy_registry(&config);
        info!(
            strategies = ?strategies.strategies().iter().map(|s| s.name()).collect::<Vec<_>>(),
            "Strategies loaded"
        );

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
        let strategies = Arc::new(strategies);

        let handler = WebSocketHandler::new(config.network.ws_url);

        let cache_clone = cache.clone();
        let registry_clone = registry.clone();
        let strategies_clone = strategies.clone();
        let executor_clone = executor.clone();

        handler
            .run(token_ids, move |msg| {
                handle_message(
                    msg,
                    &cache_clone,
                    &registry_clone,
                    &strategies_clone,
                    executor_clone.clone(),
                );
            })
            .await?;

        Ok(())
    }
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
) {
    match msg {
        WsMessage::Book(book) => {
            let orderbook = book.to_orderbook();
            let token_id = orderbook.token_id().clone();
            cache.update(orderbook);

            if let Some(pair) = registry.get_market_for_token(&token_id) {
                // Create detection context
                let ctx = DetectionContext::new(pair, cache);

                // Run all applicable strategies
                let opportunities = strategies.detect_all(&ctx);

                for opp in opportunities {
                    log_opportunity(&opp);

                    if let Some(exec) = executor.clone() {
                        spawn_execution(exec, opp);
                    }
                }
            }
        }
        WsMessage::PriceChange(_) => {}
        _ => {}
    }
}

/// Log detected opportunity.
fn log_opportunity(opp: &Opportunity) {
    info!(
        market = %opp.market_id(),
        question = %opp.question(),
        yes_ask = %opp.yes_ask(),
        no_ask = %opp.no_ask(),
        total_cost = %opp.total_cost(),
        edge = %opp.edge(),
        volume = %opp.volume(),
        expected_profit = %opp.expected_profit(),
        "ARBITRAGE DETECTED"
    );
}

/// Spawn async execution without blocking message processing.
fn spawn_execution(executor: Arc<PolymarketExecutor>, opportunity: Opportunity) {
    tokio::spawn(async move {
        match executor.execute_arbitrage(&opportunity).await {
            Ok(result) => {
                info!(result = ?result, "Execution completed");
            }
            Err(e) => {
                error!(error = %e, "Execution failed");
            }
        }
    });
}

/// Log market pair being tracked.
#[allow(dead_code)]
fn log_market(pair: &MarketPair) {
    debug!(
        market_id = %pair.market_id(),
        question = %pair.question(),
        "Tracking market"
    );
}
