use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use edgelord::app::Config;
use edgelord::error::{ConfigError, Error};

fn write_temp_config(contents: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    path.push(format!("edgelord-config-test-{nanos}.toml"));
    fs::write(&path, contents).expect("write temp config");
    path
}

#[test]
fn config_rejects_invalid_slippage() {
    let toml = r#"
exchange = "polymarket"

[exchange_config]
type = "polymarket"
ws_url = "wss://ws-subscriptions-clob.polymarket.com/ws/market"
api_url = "https://clob.polymarket.com"

[logging]
level = "info"
format = "pretty"

[risk]
max_slippage = 1.5
"#;

    let path = write_temp_config(toml);
    let result = Config::load(&path);
    let _ = fs::remove_file(&path);

    match result {
        Err(Error::Config(ConfigError::InvalidValue { field: "max_slippage", .. })) => {}
        Err(err) => panic!("Expected invalid slippage error, got {err}"),
        Ok(config) => panic!(
            "Expected invalid slippage to be rejected, got {}",
            config.risk.max_slippage
        ),
    }
}

#[test]
fn config_rejects_missing_exchange_urls() {
    let toml = r#"
exchange = "polymarket"

[exchange_config]
type = "polymarket"
ws_url = ""
api_url = ""

[logging]
level = "info"
format = "pretty"
"#;

    let path = write_temp_config(toml);
    let result = Config::load(&path);
    let _ = fs::remove_file(&path);

    assert!(
        matches!(
            result,
            Err(Error::Config(ConfigError::MissingField { field: "ws_url" }))
        ),
        "Expected missing ws_url to be rejected"
    );
}
