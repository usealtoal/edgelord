use std::path::Path;

use crate::adapter::inbound::cli::{operator, output};
use crate::error::Result;

/// Test WebSocket connection to the exchange.
pub async fn execute_connection<P: AsRef<Path>>(config_path: P) -> Result<()> {
    let service = operator::operator();
    let config_toml = operator::read_config_toml(config_path.as_ref())?;
    let target = service.connection_target(&config_toml)?;

    output::section("Connection Check");
    output::field("Exchange", target.exchange);
    output::field("WebSocket", &target.ws_url);
    output::field("API", &target.api_url);
    output::field("Environment", target.environment);

    // Test REST API connectivity
    let pb = output::spinner("Checking REST API...");
    match service.verify_rest_connectivity(&target.api_url).await {
        Ok(()) => {
            output::spinner_success(&pb, "REST API connected");
        }
        Err(e) => {
            output::spinner_fail(&pb, "REST API connection failed");
            return Err(e);
        }
    }

    // Test WebSocket connectivity
    let pb = output::spinner("Checking WebSocket...");
    match service.verify_websocket_connectivity(&target.ws_url).await {
        Ok(()) => {
            output::spinner_success(&pb, "WebSocket connected");
        }
        Err(e) => {
            output::spinner_fail(&pb, "WebSocket connection failed");
            return Err(e);
        }
    }

    output::success("Connection checks passed");

    Ok(())
}
