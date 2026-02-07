use std::path::Path;

use crate::app::Config;
use crate::error::{Error, Result};

/// Test WebSocket connection to the exchange.
pub async fn execute_connection<P: AsRef<Path>>(config_path: P) -> Result<()> {
    let config = Config::load(config_path)?;
    let network = config.network();

    println!("Testing connection to {:?}...", config.exchange);
    println!("  WebSocket: {}", network.ws_url);
    println!("  API: {}", network.api_url);
    println!("  Environment: {}", network.environment);
    println!();

    // Test REST API connectivity
    print!("Testing REST API... ");
    let client = reqwest::Client::new();
    let api_url = format!("{}/markets", network.api_url);

    match client.get(&api_url).send().await {
        Ok(response) if response.status().is_success() => {
            println!("✓ OK");
        }
        Ok(response) => {
            println!("✗ HTTP {}", response.status());
            return Err(Error::Connection(
                "API returned non-success status".to_string(),
            ));
        }
        Err(e) => {
            println!("✗ Failed");
            return Err(Error::Connection(e.to_string()));
        }
    }

    // Test WebSocket connectivity
    print!("Testing WebSocket... ");
    match tokio_tungstenite::connect_async(&network.ws_url).await {
        Ok((_, _)) => {
            println!("✓ OK");
        }
        Err(e) => {
            println!("✗ Failed");
            return Err(Error::Connection(e.to_string()));
        }
    }

    println!();
    println!("All connection tests passed.");

    Ok(())
}
