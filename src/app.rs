//! App orchestration module.
//!
//! This module contains the main application logic for running
//! the edgelord arbitrage detection and execution system.
//!
//! Requires the `polymarket` feature.

use std::sync::Arc;

use crate::config::Config;
use crate::domain::{
    detect_single_condition, DetectorConfig, MarketPair, Opportunity, OrderBookCache,
};
use crate::error::Result;
use crate::polymarket::{
    MarketRegistry, PolymarketClient, PolymarketExecutor, WebSocketHandler, WsMessage,
};
use tracing::{error, info, warn};

/// Main application struct.
pub struct App;

impl App {
    /// Run the main application loop.
    ///
    /// This initializes the executor (if wallet is configured), fetches markets,
    /// and starts the WebSocket handler for real-time order book updates.
    pub async fn run(config: Config) -> Result<()> {
        let executor = init_executor(&config).await;

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
            log_opportunity(pair);
        }

        let token_ids: Vec<String> = registry
            .pairs()
            .iter()
            .flat_map(|p| vec![p.yes_token().to_string(), p.no_token().to_string()])
            .collect();

        info!(tokens = token_ids.len(), "Subscribing to tokens");

        let cache = Arc::new(OrderBookCache::new());
        let registry = Arc::new(registry);
        let detector_config = Arc::new(config.detector.clone());

        let handler = WebSocketHandler::new(config.network.ws_url);

        let cache_clone = cache.clone();
        let registry_clone = registry.clone();
        let detector_config_clone = detector_config.clone();
        let executor_clone = executor.clone();

        handler
            .run(token_ids, move |msg| {
                handle_message(
                    msg,
                    &cache_clone,
                    &registry_clone,
                    &detector_config_clone,
                    executor_clone.clone(),
                );
            })
            .await?;

        Ok(())
    }
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
    config: &DetectorConfig,
    executor: Option<Arc<PolymarketExecutor>>,
) {
    match msg {
        WsMessage::Book(book) => {
            let orderbook = book.to_orderbook();
            let token_id = orderbook.token_id().clone();
            cache.update(orderbook);
            if let Some(pair) = registry.get_market_for_token(&token_id) {
                if let Some(opp) = detect_single_condition(pair, cache, config) {
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
fn log_opportunity(pair: &MarketPair) {
    info!(
        market_id = %pair.market_id(),
        question = %pair.question(),
        "Tracking market"
    );
}
