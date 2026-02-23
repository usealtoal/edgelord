//! Configuration projections for operator-facing adapters.

use rust_decimal::Decimal;

use crate::error::Result;

/// Risk section of a configuration view.
#[derive(Debug, Clone)]
pub struct ConfigRiskLimits {
    pub max_position_per_market: Decimal,
    pub max_total_exposure: Decimal,
    pub min_profit_threshold: Decimal,
    pub max_slippage: Decimal,
}

/// Inference section of a configuration view.
#[derive(Debug, Clone)]
pub struct ConfigInference {
    pub enabled: bool,
    pub min_confidence: f64,
    pub ttl_seconds: u64,
}

/// Cluster detection section of a configuration view.
#[derive(Debug, Clone)]
pub struct ConfigClusterDetection {
    pub enabled: bool,
    pub debounce_ms: u64,
    pub min_gap: Decimal,
}

/// Full configuration projection for operator-facing output.
#[derive(Debug, Clone)]
pub struct ConfigView {
    pub profile: String,
    pub dry_run: bool,
    pub exchange: String,
    pub environment: String,
    pub chain_id: u64,
    pub ws_url: String,
    pub api_url: String,
    pub enabled_strategies: Vec<String>,
    pub risk: ConfigRiskLimits,
    pub wallet_private_key_loaded: bool,
    pub telegram_enabled: bool,
    pub llm_provider: String,
    pub inference: ConfigInference,
    pub cluster_detection: ConfigClusterDetection,
}

/// Validation output for `config validate`.
#[derive(Debug, Clone, Default)]
pub struct ConfigValidationReport {
    pub warnings: Vec<String>,
}

/// Configuration use-cases for operator-facing adapters.
pub trait ConfigurationOperator: Send + Sync {
    /// Build a projection for `config show`.
    fn show_config(&self, config_toml: &str) -> Result<ConfigView>;

    /// Validate config and return non-fatal warnings.
    fn validate_config(&self, config_toml: &str) -> Result<ConfigValidationReport>;
}
