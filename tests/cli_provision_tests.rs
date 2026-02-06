use edgelord::cli::{Cli, Commands};
use edgelord::cli::provision::{execute, ProvisionCommand, ProvisionPolymarketArgs, WalletMode};
use clap::Parser;
use std::fs;
use std::path::PathBuf;
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
