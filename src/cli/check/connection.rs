use std::path::Path;

use crate::cli::output;
use crate::error::{Error, Result};
use crate::runtime::Config;

/// Test WebSocket connection to the exchange.
pub async fn execute_connection<P: AsRef<Path>>(config_path: P) -> Result<()> {
    let config = Config::load(config_path)?;
    let network = config.network();

    output::section("Connection Check");
    output::field("Exchange", format!("{:?}", config.exchange));
    output::field("WebSocket", &network.ws_url);
    output::field("API", &network.api_url);
    output::field("Environment", network.environment);

    // Test REST API connectivity
    let pb = output::spinner("REST API connectivity");
    let client = reqwest::Client::new();
    let api_url = format!("{}/markets", network.api_url);

    match client.get(&api_url).send().await {
        Ok(response) if response.status().is_success() => {
            output::spinner_success(&pb, "REST API connectivity");
        }
        Ok(response) => {
            output::spinner_fail(&pb, "REST API connectivity");
            return Err(Error::Connection(format!(
                "REST API returned non-success status: {}",
                response.status()
            )));
        }
        Err(e) => {
            output::spinner_fail(&pb, "REST API connectivity");
            return Err(Error::Connection(e.to_string()));
        }
    }

    // Test WebSocket connectivity
    let pb = output::spinner("WebSocket connectivity");
    match tokio_tungstenite::connect_async(&network.ws_url).await {
        Ok((_, _)) => {
            output::spinner_success(&pb, "WebSocket connectivity");
        }
        Err(e) => {
            output::spinner_fail(&pb, "WebSocket connectivity");
            return Err(Error::Connection(e.to_string()));
        }
    }

    output::success("Connection checks passed");

    Ok(())
}
