//! Exchange provisioning commands.

use std::fs;
use std::path::{Path, PathBuf};

use alloy_signer_local::PrivateKeySigner;
use clap::{Parser, Subcommand, ValueEnum};
use rand::rngs::OsRng;

use crate::error::{ConfigError, Result};

/// Subcommands for `edgelord provision`.
#[derive(Subcommand, Debug)]
pub enum ProvisionCommand {
    /// Provision Polymarket configuration
    Polymarket(ProvisionPolymarketArgs),
}

/// Wallet setup mode.
#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum WalletMode {
    /// Generate a new wallet and encrypted keystore
    Generate,
    /// Import a private key into an encrypted keystore
    Import,
}

/// Arguments for `edgelord provision polymarket`.
#[derive(Parser, Debug)]
pub struct ProvisionPolymarketArgs {
    /// Path to configuration file
    #[arg(short, long, default_value = "config.polymarket.toml")]
    pub config: PathBuf,

    /// Wallet setup mode
    #[arg(long, value_enum, default_value = "generate")]
    pub wallet: WalletMode,

    /// Keystore file path
    #[arg(long)]
    pub keystore_path: Option<PathBuf>,
}

/// Execute provisioning command.
pub async fn execute(command: ProvisionCommand) -> Result<()> {
    match command {
        ProvisionCommand::Polymarket(args) => provision_polymarket(args),
    }
}

fn provision_polymarket(args: ProvisionPolymarketArgs) -> Result<()> {
    let config_path = args.config;
    let keystore_path = args
        .keystore_path
        .unwrap_or_else(default_polymarket_keystore_path);

    ensure_config_exists(&config_path)?;
    ensure_polymarket_config(&config_path)?;

    if keystore_path.exists() {
        return Err(ConfigError::InvalidValue {
            field: "keystore_path",
            reason: "keystore already exists".to_string(),
        }
        .into());
    }

    let password = read_keystore_password()?;
    let signer = match args.wallet {
        WalletMode::Generate => create_keystore(&keystore_path, &password)?,
        WalletMode::Import => {
            return Err(ConfigError::InvalidValue {
                field: "wallet",
                reason: "import mode not implemented yet".to_string(),
            }
            .into());
        }
    };

    update_wallet_keystore_path(&config_path, &keystore_path)?;

    println!("Provisioned Polymarket wallet");
    println!("  Address: {}", signer.address());
    println!("  Keystore: {}", keystore_path.display());
    println!("  Funding: USDC on Polygon");

    Ok(())
}

fn ensure_config_exists(path: &Path) -> Result<()> {
    if path.exists() {
        return Ok(());
    }

    let template = include_str!("../../../config.toml.example");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, template)?;
    Ok(())
}

fn ensure_polymarket_config(path: &Path) -> Result<()> {
    let contents = fs::read_to_string(path).map_err(ConfigError::ReadFile)?;
    let mut config: toml::Value = toml::from_str(&contents).map_err(ConfigError::Parse)?;

    let table = config
        .as_table_mut()
        .ok_or_else(|| ConfigError::Other("config not a table".to_string()))?;

    table.insert(
        "exchange".to_string(),
        toml::Value::String("polymarket".to_string()),
    );

    let exchange_config = table
        .entry("exchange_config")
        .or_insert_with(|| toml::Value::Table(Default::default()));
    if let Some(exchange_table) = exchange_config.as_table_mut() {
        exchange_table.insert(
            "type".to_string(),
            toml::Value::String("polymarket".to_string()),
        );
    }

    let rendered = toml::to_string_pretty(&config)
        .map_err(|e| ConfigError::Other(e.to_string()))?;
    fs::write(path, rendered)?;
    Ok(())
}

fn update_wallet_keystore_path(path: &Path, keystore_path: &Path) -> Result<()> {
    let contents = fs::read_to_string(path).map_err(ConfigError::ReadFile)?;
    let mut config: toml::Value = toml::from_str(&contents).map_err(ConfigError::Parse)?;

    let table = config
        .as_table_mut()
        .ok_or_else(|| ConfigError::Other("config not a table".to_string()))?;

    let wallet = table
        .entry("wallet")
        .or_insert_with(|| toml::Value::Table(Default::default()));
    if let Some(wallet_table) = wallet.as_table_mut() {
        wallet_table.insert(
            "keystore_path".to_string(),
            toml::Value::String(keystore_path.to_string_lossy().to_string()),
        );
    }

    let rendered = toml::to_string_pretty(&config)
        .map_err(|e| ConfigError::Other(e.to_string()))?;
    fs::write(path, rendered)?;
    Ok(())
}

fn read_keystore_password() -> Result<String> {
    if let Ok(password) = std::env::var("EDGELORD_KEYSTORE_PASSWORD") {
        return Ok(password);
    }
    if let Ok(path) = std::env::var("EDGELORD_KEYSTORE_PASSWORD_FILE") {
        let contents = fs::read_to_string(path).map_err(ConfigError::ReadFile)?;
        let password = contents.trim().to_string();
        if password.is_empty() {
            return Err(ConfigError::MissingField {
                field: "EDGELORD_KEYSTORE_PASSWORD_FILE",
            }
            .into());
        }
        return Ok(password);
    }

    Err(ConfigError::MissingField {
        field: "EDGELORD_KEYSTORE_PASSWORD",
    }
    .into())
}

fn create_keystore(path: &Path, password: &str) -> Result<PrivateKeySigner> {
    let parent = path.parent().ok_or_else(|| ConfigError::InvalidValue {
        field: "keystore_path",
        reason: "missing parent directory".to_string(),
    })?;
    fs::create_dir_all(parent)?;

    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| ConfigError::InvalidValue {
            field: "keystore_path",
            reason: "invalid file name".to_string(),
        })?;

    let mut rng = OsRng;
    let (signer, _uuid) = PrivateKeySigner::new_keystore(parent, &mut rng, password, Some(name))
        .map_err(|e| ConfigError::InvalidValue {
            field: "keystore_path",
            reason: e.to_string(),
        })?;

    Ok(signer)
}

fn default_polymarket_keystore_path() -> PathBuf {
    let base = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    base.join(".config/edgelord/exchanges/polymarket/keystore.json")
}
