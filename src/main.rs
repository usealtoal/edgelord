mod config;
mod domain;
mod error;
mod polymarket;

use std::sync::Arc;

use config::Config;
use domain::{detect_single_condition, DetectorConfig, OrderBookCache, TokenId};
use polymarket::{MarketRegistry, PolymarketClient, WebSocketHandler, WsMessage};
use tokio::signal;
use tracing::{error, info, warn};

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
        info!(
            market_id = %pair.market_id,
            question = %pair.question,
            "Tracking market"
        );
    }

    let token_ids: Vec<String> = registry
        .pairs()
        .iter()
        .flat_map(|p| vec![p.yes_token.0.clone(), p.no_token.0.clone()])
        .collect();

    info!(tokens = token_ids.len(), "Subscribing to tokens");

    let cache = Arc::new(OrderBookCache::new());
    let registry = Arc::new(registry);
    let detector_config = Arc::new(config.detector.clone());

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
            cache.update_from_ws(&book);

            let token_id = TokenId::from(book.asset_id.clone());
            if let Some(pair) = registry.get_market_for_token(&token_id) {
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
        WsMessage::PriceChange(_) => {}
        _ => {}
    }
}
