mod api;
mod config;
mod detector;
mod error;
mod orderbook;
mod types;
mod websocket;

use std::sync::Arc;

use api::ApiClient;
use config::Config;
use detector::{detect_single_condition, DetectorConfig};
use orderbook::{MarketRegistry, OrderBookCache};
use tokio::signal;
use tracing::{error, info, warn};
use websocket::{WebSocketHandler, WsMessage};

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    let config = match Config::load("config.toml") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    };

    config.init_logging();

    info!("edgelord starting");

    tokio::select! {
        result = run(config) => {
            if let Err(e) = result {
                error!(error = %e, "Fatal error");
                std::process::exit(1);
            }
        }
        _ = signal::ctrl_c() => {
            info!("Shutdown signal received");
        }
    }

    info!("edgelord stopped");
}

async fn run(config: Config) -> error::Result<()> {
    // Fetch active markets
    let api = ApiClient::new(config.network.api_url.clone());
    let markets = api.get_active_markets(20).await?;

    if markets.is_empty() {
        warn!("No active markets found");
        return Ok(());
    }

    // Build market registry (only YES/NO pairs)
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

    // Log the pairs we're tracking
    for pair in registry.pairs() {
        info!(
            market_id = %pair.market_id,
            question = %pair.question,
            "Tracking market"
        );
    }

    // Collect all token IDs to subscribe to
    let token_ids: Vec<String> = registry
        .pairs()
        .iter()
        .flat_map(|p| vec![p.yes_token.0.clone(), p.no_token.0.clone()])
        .collect();

    info!(tokens = token_ids.len(), "Subscribing to tokens");

    // Create shared state
    let cache = Arc::new(OrderBookCache::new());
    let registry = Arc::new(registry);
    let detector_config = Arc::new(config.detector.clone());

    // Connect to WebSocket and process messages
    let handler = WebSocketHandler::new(config.network.ws_url);

    let cache_clone = cache.clone();
    let registry_clone = registry.clone();
    let detector_config_clone = detector_config.clone();

    handler
        .run(token_ids, move |msg| {
            handle_message(msg, &cache_clone, &registry_clone, &detector_config_clone);
        })
        .await?;

    Ok(())
}

fn handle_message(
    msg: WsMessage,
    cache: &OrderBookCache,
    registry: &MarketRegistry,
    config: &DetectorConfig,
) {
    match msg {
        WsMessage::Book(book) => {
            // Update cache
            cache.update(&book);

            // Get the market for this token
            let token_id = types::TokenId::from(book.asset_id.clone());
            if let Some(pair) = registry.get_market_for_token(&token_id) {
                // Check for arbitrage on this market
                if let Some(opp) = detect_single_condition(pair, cache, config) {
                    info!(
                        market = %opp.market_id,
                        question = %opp.question,
                        yes_ask = %opp.yes_ask,
                        no_ask = %opp.no_ask,
                        total_cost = %opp.total_cost,
                        edge = %opp.edge,
                        volume = %opp.volume,
                        expected_profit = %opp.expected_profit,
                        "ARBITRAGE DETECTED"
                    );
                }
            }
        }
        WsMessage::PriceChange(_) => {
            // Price changes don't have full book data, ignore for now
        }
        _ => {}
    }
}
