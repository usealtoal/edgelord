// These tests intentionally hold a MutexGuard across await to serialize access
// to environment variables shared between parallel tests.
#![allow(clippy::await_holding_lock)]

use alloy_signer_local::PrivateKeySigner;
use edgelord::cli::provision::{execute, ProvisionCommand, ProvisionPolymarketArgs, WalletMode};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn temp_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    path.push(format!("edgelord-wallet-{name}-{nanos}"));
    path
}

#[tokio::test]
async fn wallet_address_reads_from_keystore() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");

    let dir = temp_path("address");
    fs::create_dir_all(&dir).expect("create temp dir");
    let config_path = dir.join("config.toml");
    let keystore_path = dir.join("keystore.json");

    let template = include_str!("../config.toml.example");
    fs::write(&config_path, template).expect("write config template");

    std::env::set_var("EDGELORD_KEYSTORE_PASSWORD", "test-password");

    let args = ProvisionPolymarketArgs {
        config: config_path.clone(),
        wallet: WalletMode::Generate,
        keystore_path: Some(keystore_path.clone()),
    };

    execute(ProvisionCommand::Polymarket(args))
        .await
        .expect("provision polymarket");

    let signer = PrivateKeySigner::decrypt_keystore(&keystore_path, "test-password")
        .expect("decrypt keystore");
    let expected_address = signer.address().to_string();

    let output = Command::new(env!("CARGO_BIN_EXE_edgelord"))
        .args(["wallet", "address", "--config"])
        .arg(&config_path)
        .output()
        .expect("run edgelord");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "expected success");
    assert!(
        stdout.contains(&expected_address),
        "expected address in stdout: {stdout}"
    );

    let _ = fs::remove_dir_all(&dir);
    std::env::remove_var("EDGELORD_KEYSTORE_PASSWORD");
}

#[test]
fn wallet_sweep_requires_wallet() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");

    let template = include_str!("../config.toml.example");
    let path = temp_path("sweep");
    fs::write(&path, template).expect("write config template");

    std::env::remove_var("WALLET_PRIVATE_KEY");
    std::env::remove_var("EDGELORD_KEYSTORE_PASSWORD");
    std::env::remove_var("EDGELORD_KEYSTORE_PASSWORD_FILE");

    let output = Command::new(env!("CARGO_BIN_EXE_edgelord"))
        .args([
            "wallet",
            "sweep",
            "--to",
            "0x0000000000000000000000000000000000000000",
            "--yes",
            "--config",
        ])
        .arg(&path)
        .output()
        .expect("run edgelord");

    let _ = fs::remove_file(&path);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "expected nonzero exit when wallet missing"
    );
    assert!(
        stderr.contains("WALLET_PRIVATE_KEY"),
        "expected missing wallet error, got: {stderr}"
    );
}
