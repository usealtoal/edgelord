mod api;
mod config;
mod error;
mod types;
mod websocket;

use api::ApiClient;
use config::Config;
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
    let api = ApiClient::new(config.network.api_url.clone());
    let markets = api.get_active_markets(5).await?;

    if markets.is_empty() {
        warn!("No active markets found");
        return Ok(());
    }

    let token_ids: Vec<String> = markets.iter().flat_map(|m| m.token_ids()).collect();

    info!(
        markets = markets.len(),
        tokens = token_ids.len(),
        "Subscribing to markets"
    );

    for market in &markets {
        info!(
            condition_id = %market.condition_id,
            question = ?market.question,
            "Market"
        );
    }

    let handler = WebSocketHandler::new(config.network.ws_url);

    handler
        .run(token_ids, |msg| match msg {
            WsMessage::Book(book) => {
                let best_bid = book.bids.first().map(|b| b.price.as_str()).unwrap_or("-");
                let best_ask = book.asks.first().map(|a| a.price.as_str()).unwrap_or("-");

                info!(
                    asset = %book.asset_id,
                    bid = %best_bid,
                    ask = %best_ask,
                    "Book"
                );
            }
            WsMessage::PriceChange(change) => {
                info!(
                    asset = %change.asset_id,
                    price = ?change.price,
                    "Price"
                );
            }
            _ => {}
        })
        .await?;

    Ok(())
}
