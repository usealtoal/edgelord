use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use alloy_signer_local::PrivateKeySigner;
use clap::{Parser, ValueEnum};
use rand::rngs::OsRng;

use crate::adapter::inbound::cli::output;
use crate::error::{ConfigError, Result};

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
    #[arg(short, long, default_value_os_t = crate::adapter::inbound::cli::paths::default_config())]
    pub config: PathBuf,

    /// Wallet setup mode
    #[arg(long, value_enum, default_value = "generate")]
    pub wallet: WalletMode,

    /// Keystore file path
    #[arg(long)]
    pub keystore_path: Option<PathBuf>,
}

pub(super) fn execute_polymarket(args: ProvisionPolymarketArgs) -> Result<()> {
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
        WalletMode::Import => import_keystore(&keystore_path, &password)?,
    };

    update_wallet_keystore_path(&config_path, &keystore_path)?;

    output::section("Polymarket Provisioning");
    output::success("Wallet provisioned");
    output::field("Address", signer.address());
    output::field("Keystore", keystore_path.display());
    output::field("Funding", "USDC on Polygon");

    Ok(())
}

fn ensure_config_exists(path: &Path) -> Result<()> {
    if path.exists() {
        return Ok(());
    }

    let template = include_str!("../../../../../config.toml.example");
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

    let rendered =
        toml::to_string_pretty(&config).map_err(|e| ConfigError::Other(e.to_string()))?;
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

    let rendered =
        toml::to_string_pretty(&config).map_err(|e| ConfigError::Other(e.to_string()))?;
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

    let name =
        path.file_name()
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

fn import_keystore(path: &Path, password: &str) -> Result<PrivateKeySigner> {
    let private_key =
        std::env::var("EDGELORD_PRIVATE_KEY").map_err(|_| ConfigError::MissingField {
            field: "EDGELORD_PRIVATE_KEY",
        })?;
    let signer =
        PrivateKeySigner::from_str(&private_key).map_err(|e| ConfigError::InvalidValue {
            field: "EDGELORD_PRIVATE_KEY",
            reason: e.to_string(),
        })?;

    let parent = path.parent().ok_or_else(|| ConfigError::InvalidValue {
        field: "keystore_path",
        reason: "missing parent directory".to_string(),
    })?;
    fs::create_dir_all(parent)?;

    let name =
        path.file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| ConfigError::InvalidValue {
                field: "keystore_path",
                reason: "invalid file name".to_string(),
            })?;

    let mut rng = OsRng;
    let (signer, _uuid) = PrivateKeySigner::encrypt_keystore(
        parent,
        &mut rng,
        signer.to_bytes(),
        password,
        Some(name),
    )
    .map_err(|e| ConfigError::InvalidValue {
        field: "keystore_path",
        reason: e.to_string(),
    })?;

    Ok(signer)
}

fn default_polymarket_keystore_path() -> PathBuf {
    crate::adapter::inbound::cli::paths::keystore_dir().join("polymarket.json")
}
