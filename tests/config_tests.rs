use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use edgelord::error::{ConfigError, Error};
use edgelord::infrastructure::config::settings::Config;

static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn write_temp_config(contents: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let suffix = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    path.push(format!("edgelord-config-test-{nanos}-{suffix}.toml"));
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
        Err(Error::Config(ConfigError::InvalidValue {
            field: "max_slippage",
            ..
        })) => {}
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

#[test]
fn config_rejects_negative_risk_limits() {
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
max_position_per_market = -1
"#;

    let path = write_temp_config(toml);
    let result = Config::load(&path);
    let _ = fs::remove_file(&path);

    match result {
        Err(Error::Config(ConfigError::InvalidValue {
            field: "max_position_per_market",
            ..
        })) => {}
        Err(err) => panic!("Expected invalid risk limit error, got {err}"),
        Ok(_) => panic!("Expected invalid risk limit to be rejected"),
    }
}

#[test]
fn config_rejects_invalid_reconnection_backoff() {
    let toml = r#"
exchange = "polymarket"

[exchange_config]
type = "polymarket"
ws_url = "wss://ws-subscriptions-clob.polymarket.com/ws/market"
api_url = "https://clob.polymarket.com"

[logging]
level = "info"
format = "pretty"

[reconnection]
backoff_multiplier = 0.5
"#;

    let path = write_temp_config(toml);
    let result = Config::load(&path);
    let _ = fs::remove_file(&path);

    match result {
        Err(Error::Config(ConfigError::InvalidValue {
            field: "backoff_multiplier",
            ..
        })) => {}
        Err(err) => panic!("Expected invalid backoff error, got {err}"),
        Ok(_) => panic!("Expected invalid backoff to be rejected"),
    }
}

#[test]
fn config_rejects_invalid_latency_targets() {
    let toml = r#"
exchange = "polymarket"

[exchange_config]
type = "polymarket"
ws_url = "wss://ws-subscriptions-clob.polymarket.com/ws/market"
api_url = "https://clob.polymarket.com"

[logging]
level = "info"
format = "pretty"

[governor.latency]
target_p50_ms = 100
target_p95_ms = 50
"#;

    let path = write_temp_config(toml);
    let result = Config::load(&path);
    let _ = fs::remove_file(&path);

    match result {
        Err(Error::Config(ConfigError::InvalidValue {
            field: "latency_targets",
            ..
        })) => {}
        Err(err) => panic!("Expected invalid latency targets error, got {err}"),
        Ok(_) => panic!("Expected invalid latency targets to be rejected"),
    }
}

#[test]
fn config_rejects_invalid_cluster_min_gap() {
    let toml = r#"
exchange = "polymarket"

[exchange_config]
type = "polymarket"
ws_url = "wss://ws-subscriptions-clob.polymarket.com/ws/market"
api_url = "https://clob.polymarket.com"

[logging]
level = "info"
format = "pretty"

[cluster_detection]
enabled = true
min_gap = 1.5
"#;

    let path = write_temp_config(toml);
    let result = Config::load(&path);
    let _ = fs::remove_file(&path);

    match result {
        Err(Error::Config(ConfigError::InvalidValue {
            field: "min_gap", ..
        })) => {}
        Err(err) => panic!("Expected invalid min_gap error, got {err}"),
        Ok(_) => panic!("Expected invalid min_gap to be rejected"),
    }
}
