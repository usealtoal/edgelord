use alloy_signer_local::PrivateKeySigner;
use clap::Parser;
use edgelord::adapter::inbound::cli::command::{Cli, Commands};
use edgelord::adapter::inbound::cli::provision::command::{execute, ProvisionCommand};
use edgelord::adapter::inbound::cli::provision::polymarket::{ProvisionPolymarketArgs, WalletMode};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn provision_command_is_registered() {
    let cli = Cli::parse_from(["edgelord", "provision", "polymarket"]);
    match cli.command {
        Commands::Provision(_) => {}
        _ => panic!("expected provision subcommand"),
    }
}

fn temp_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    path.push(format!("edgelord-{name}-{nanos}"));
    path
}

#[tokio::test]
async fn provision_polymarket_writes_keystore_and_updates_config() {
    let dir = temp_path("provision");
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

    assert!(keystore_path.exists(), "expected keystore to be created");

    let contents = fs::read_to_string(&config_path).expect("read config");
    let config: toml::Value = toml::from_str(&contents).expect("parse config");
    let wallet = config
        .get("wallet")
        .expect("wallet section")
        .as_table()
        .expect("wallet table");
    let path_value = wallet
        .get("keystore_path")
        .expect("keystore_path")
        .as_str()
        .expect("keystore_path string");
    assert_eq!(path_value, keystore_path.to_string_lossy());

    let _ = fs::remove_dir_all(&dir);
    std::env::remove_var("EDGELORD_KEYSTORE_PASSWORD");
}

#[tokio::test]
async fn provision_polymarket_imports_private_key() {
    let dir = temp_path("provision-import");
    fs::create_dir_all(&dir).expect("create temp dir");
    let config_path = dir.join("config.toml");
    let keystore_path = dir.join("keystore.json");

    let template = include_str!("../config.toml.example");
    fs::write(&config_path, template).expect("write config template");

    let private_key = "6f142508b4eea641e33cb2a0161221105086a84584c74245ca463a49effea30b";
    std::env::set_var("EDGELORD_PRIVATE_KEY", private_key);
    std::env::set_var("EDGELORD_KEYSTORE_PASSWORD", "test-password");

    let args = ProvisionPolymarketArgs {
        config: config_path.clone(),
        wallet: WalletMode::Import,
        keystore_path: Some(keystore_path.clone()),
    };

    execute(ProvisionCommand::Polymarket(args))
        .await
        .expect("provision polymarket import");

    assert!(keystore_path.exists(), "expected keystore to be created");

    let expected = PrivateKeySigner::from_str(private_key).expect("parse private key");
    let signer = PrivateKeySigner::decrypt_keystore(&keystore_path, "test-password")
        .expect("decrypt keystore");
    assert_eq!(signer.address(), expected.address());

    let _ = fs::remove_dir_all(&dir);
    std::env::remove_var("EDGELORD_PRIVATE_KEY");
    std::env::remove_var("EDGELORD_KEYSTORE_PASSWORD");
}
