use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn write_temp_config(contents: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    path.push(format!("edgelord-cli-test-{nanos}.toml"));
    fs::write(&path, contents).expect("write temp config");
    path
}

#[test]
fn cli_returns_nonzero_on_config_error() {
    let toml = concat!(
        "exchange = \"polymarket\"\n",
        "\n",
        "[exchange_config]\n",
        "type = \"polymarket\"\n",
        "ws_url = \"wss://ws-subscriptions-clob.polymarket.com/ws/market\"\n",
        "api_url = \"https://clob.polymarket.com\"\n",
        "\n",
        "[logging]\n",
        "level = \"info\"\n",
        "format = \"pretty\"\n",
        "\n",
        "[risk]\n",
        "max_slippage = 1.5\n",
    );

    let path = write_temp_config(toml);
    let output = Command::new(env!("CARGO_BIN_EXE_edgelord"))
        .args(["config", "validate", "--config"])
        .arg(&path)
        .output()
        .expect("run edgelord");
    let _ = fs::remove_file(&path);

    assert!(!output.status.success(), "Expected nonzero exit code");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Error:"),
        "Expected standardized error prefix"
    );
    assert!(
        !stderr.contains("Config file is invalid"),
        "Expected normalized error output without extra banners"
    );
}
