//! Wallet configuration for signing orders.

use serde::Deserialize;

/// Wallet configuration for signing orders.
/// Private key is loaded from `WALLET_PRIVATE_KEY` env var at runtime (never from config file).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct WalletConfig {
    /// Optional keystore path for encrypted wallet storage.
    #[serde(default)]
    pub keystore_path: Option<String>,
    /// Private key loaded from `WALLET_PRIVATE_KEY` env var at runtime
    #[serde(skip)]
    pub private_key: Option<String>,
}
