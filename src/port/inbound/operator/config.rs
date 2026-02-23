//! Configuration projection types for operator-facing adapters.
//!
//! Defines view models for displaying and validating configuration settings
//! through operator interfaces like the CLI.

use rust_decimal::Decimal;

use crate::error::Result;

/// Risk limits section of a configuration view.
#[derive(Debug, Clone)]
pub struct ConfigRiskLimits {
    /// Maximum position size per market in USD.
    pub max_position_per_market: Decimal,

    /// Maximum total exposure across all positions in USD.
    pub max_total_exposure: Decimal,

    /// Minimum profit threshold for trade execution in USD.
    pub min_profit_threshold: Decimal,

    /// Maximum acceptable slippage as a decimal.
    pub max_slippage: Decimal,
}

/// LLM inference section of a configuration view.
#[derive(Debug, Clone)]
pub struct ConfigInference {
    /// Whether LLM inference is enabled.
    pub enabled: bool,

    /// Minimum confidence threshold for accepting inferred relations.
    pub min_confidence: f64,

    /// Time-to-live for cached inference results in seconds.
    pub ttl_seconds: u64,
}

/// Cluster detection section of a configuration view.
#[derive(Debug, Clone)]
pub struct ConfigClusterDetection {
    /// Whether cluster detection is enabled.
    pub enabled: bool,

    /// Debounce interval for cluster updates in milliseconds.
    pub debounce_ms: u64,

    /// Minimum gap threshold for triggering cluster arbitrage.
    pub min_gap: Decimal,
}

/// Complete configuration projection for operator-facing display.
///
/// Contains all configuration settings needed for the `config show` command.
#[derive(Debug, Clone)]
pub struct ConfigView {
    /// Configuration profile name.
    pub profile: String,

    /// Whether dry-run mode is enabled (no real trades).
    pub dry_run: bool,

    /// Exchange name (e.g., "polymarket").
    pub exchange: String,

    /// Environment name (e.g., "mainnet", "testnet").
    pub environment: String,

    /// Blockchain chain ID.
    pub chain_id: u64,

    /// WebSocket endpoint URL.
    pub ws_url: String,

    /// REST API endpoint URL.
    pub api_url: String,

    /// Names of enabled detection strategies.
    pub enabled_strategies: Vec<String>,

    /// Risk limit settings.
    pub risk: ConfigRiskLimits,

    /// Whether a wallet private key is configured.
    pub wallet_private_key_loaded: bool,

    /// Whether Telegram notifications are enabled.
    pub telegram_enabled: bool,

    /// Name of the configured LLM provider.
    pub llm_provider: String,

    /// LLM inference settings.
    pub inference: ConfigInference,

    /// Cluster detection settings.
    pub cluster_detection: ConfigClusterDetection,
}

/// Validation report for configuration files.
#[derive(Debug, Clone, Default)]
pub struct ConfigValidationReport {
    /// Non-fatal warnings discovered during validation.
    pub warnings: Vec<String>,
}

/// Configuration use-cases for operator-facing adapters.
///
/// # Thread Safety
///
/// Implementations must be thread-safe (`Send + Sync`).
pub trait ConfigurationOperator: Send + Sync {
    /// Build a configuration view for display.
    ///
    /// # Arguments
    ///
    /// * `config_toml` - Raw TOML configuration content.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration cannot be parsed.
    fn show_config(&self, config_toml: &str) -> Result<ConfigView>;

    /// Validate configuration and return warnings.
    ///
    /// # Arguments
    ///
    /// * `config_toml` - Raw TOML configuration content.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration has fatal validation errors.
    fn validate_config(&self, config_toml: &str) -> Result<ConfigValidationReport>;
}
