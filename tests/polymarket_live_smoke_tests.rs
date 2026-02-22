use std::env;
use std::time::Duration;

use edgelord::adapter::polymarket::PolymarketClient;
use tokio::time::timeout;

fn smoke_enabled() -> bool {
    matches!(env::var("EDGELORD_SMOKE").ok().as_deref(), Some("1"))
}

#[tokio::test]
#[ignore = "requires EDGELORD_SMOKE=1 and network access"]
async fn smoke_polymarket_rest_markets_readonly() {
    if !smoke_enabled() {
        eprintln!("Skipping smoke test (set EDGELORD_SMOKE=1 to enable)");
        return;
    }

    let base_url = env::var("POLYMARKET_API_URL")
        .unwrap_or_else(|_| "https://clob.polymarket.com".to_string());
    let client = PolymarketClient::new(base_url.clone());

    let markets = timeout(Duration::from_secs(20), client.get_active_markets(5))
        .await
        .expect("Timed out querying Polymarket markets endpoint")
        .expect("Failed to fetch active markets");

    assert!(
        !markets.is_empty(),
        "Expected at least one active market from {base_url}"
    );
}
