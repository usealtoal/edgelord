mod api;
mod config;
mod error;
mod websocket;

use api::ApiClient;
use config::Config;
use tracing::{error, info};
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

    if let Err(e) = run(config).await {
        error!(error = %e, "Fatal error");
        std::process::exit(1);
    }
}

async fn run(config: Config) -> error::Result<()> {
    // Fetch some active markets
    let api = ApiClient::new(config.network.api_url.clone());
    let markets = api.get_active_markets(5).await?;

    if markets.is_empty() {
        info!("No active markets found");
        return Ok(());
    }

    // Collect token IDs to subscribe to
    let token_ids: Vec<String> = markets
        .iter()
        .flat_map(|m| m.token_ids())
        .collect();

    info!(
        markets = markets.len(),
        tokens = token_ids.len(),
        "Subscribing to markets"
    );

    for market in &markets {
        info!(
            condition_id = %market.condition_id,
            question = ?market.question,
            tokens = market.tokens.len(),
            "Market"
        );
    }

    // Connect to WebSocket and listen
    let handler = WebSocketHandler::new(config.network.ws_url);

    handler.run(token_ids, |msg| {
        match msg {
            WsMessage::Book(book) => {
                info!(
                    asset_id = %book.asset_id,
                    bids = book.bids.len(),
                    asks = book.asks.len(),
                    "Order book snapshot"
                );

                if let Some(best_bid) = book.bids.first() {
                    info!(
                        asset_id = %book.asset_id,
                        price = %best_bid.price,
                        size = %best_bid.size,
                        "Best bid"
                    );
                }
                if let Some(best_ask) = book.asks.first() {
                    info!(
                        asset_id = %book.asset_id,
                        price = %best_ask.price,
                        size = %best_ask.size,
                        "Best ask"
                    );
                }
            }
            WsMessage::PriceChange(change) => {
                info!(
                    asset_id = %change.asset_id,
                    price = ?change.price,
                    "Price change"
                );
            }
            WsMessage::TickSizeChange(_) => {
                info!("Tick size change");
            }
            WsMessage::Unknown => {
                // Ignore unknown messages
            }
        }
    }).await?;

    Ok(())
}
