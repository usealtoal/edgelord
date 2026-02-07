use std::path::Path;

use crate::app::Config;
use crate::cli::output;
use crate::error::{Error, Result};

/// Test WebSocket connection to the exchange.
pub async fn execute_connection<P: AsRef<Path>>(config_path: P) -> Result<()> {
    let config = Config::load(config_path)?;
    let network = config.network();

    output::section("Connection Check");
    output::key_value("Exchange", format!("{:?}", config.exchange));
    output::key_value("WebSocket", &network.ws_url);
    output::key_value("API", &network.api_url);
    output::key_value("Environment", network.environment);

    // Test REST API connectivity
    print!("REST API connectivity... ");
    let client = reqwest::Client::new();
    let api_url = format!("{}/markets", network.api_url);

    match client.get(&api_url).send().await {
        Ok(response) if response.status().is_success() => {
            println!("ok");
        }
        Ok(response) => {
            println!("failed");
            return Err(Error::Connection(format!(
                "REST API returned non-success status: {}",
                response.status()
            )));
        }
        Err(e) => {
            println!("failed");
            return Err(Error::Connection(e.to_string()));
        }
    }

    // Test WebSocket connectivity
    print!("WebSocket connectivity... ");
    match tokio_tungstenite::connect_async(&network.ws_url).await {
        Ok((_, _)) => {
            println!("ok");
        }
        Err(e) => {
            println!("failed");
            return Err(Error::Connection(e.to_string()));
        }
    }

    output::ok("Connection checks passed");

    Ok(())
}
