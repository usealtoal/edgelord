//! Wallet configuration for signing orders.
//!
//! Provides configuration for wallet-based order signing. Private keys are
//! never stored in configuration files for security.

use serde::Deserialize;

/// Wallet configuration for signing orders.
///
/// The private key is loaded from the `WALLET_PRIVATE_KEY` environment
/// variable at runtime, or decrypted from a keystore file if configured.
/// Private keys are never stored in configuration files.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct WalletConfig {
    /// Path to an encrypted keystore file.
    ///
    /// When set, the keystore is decrypted using the password from
    /// `EDGELORD_KEYSTORE_PASSWORD` or `EDGELORD_KEYSTORE_PASSWORD_FILE`.
    /// Takes precedence over `WALLET_PRIVATE_KEY` if both are set.
    #[serde(default)]
    pub keystore_path: Option<String>,

    /// Private key for order signing.
    ///
    /// Loaded from `WALLET_PRIVATE_KEY` environment variable at runtime.
    /// Never serialized or stored in configuration files.
    #[serde(skip)]
    pub private_key: Option<String>,
}
