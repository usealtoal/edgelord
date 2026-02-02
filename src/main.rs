mod config;
mod error;
mod websocket;

use config::Config;
use tracing::info;

fn main() {
    let _ = dotenvy::dotenv();

    let config = match Config::load("config.toml") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    };

    config.init_logging();

    info!(ws_url = %config.network.ws_url, "edgelord starting");
}
