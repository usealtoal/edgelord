//! Runtime control port for operator-facing adapters.
//!
//! This keeps control surfaces (for example Telegram commands) decoupled
//! from concrete application state implementations.

use rust_decimal::Decimal;

use crate::domain::cluster::Cluster;
use crate::domain::money::Price;

/// Risk limits snapshot used by runtime control adapters.
#[derive(Debug, Clone)]
pub struct RuntimeRiskLimits {
    /// Maximum position size per market in dollars.
    pub max_position_per_market: Decimal,
    /// Maximum total exposure across all positions.
    pub max_total_exposure: Decimal,
    /// Minimum profit threshold to execute.
    pub min_profit_threshold: Decimal,
    /// Maximum slippage tolerance (for example 0.02 = 2%).
    pub max_slippage: Decimal,
}

/// Runtime risk limit field names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeRiskLimitKind {
    MaxPositionPerMarket,
    MaxTotalExposure,
    MinProfitThreshold,
    MaxSlippage,
}

impl RuntimeRiskLimitKind {
    /// Stable field name used in logs and command output.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MaxPositionPerMarket => "max_position",
            Self::MaxTotalExposure => "max_exposure",
            Self::MinProfitThreshold => "min_profit",
            Self::MaxSlippage => "max_slippage",
        }
    }
}

/// Error returned when a runtime risk update is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeRiskLimitUpdateError {
    reason: &'static str,
}

impl RuntimeRiskLimitUpdateError {
    #[must_use]
    pub const fn new(reason: &'static str) -> Self {
        Self { reason }
    }
}

impl std::fmt::Display for RuntimeRiskLimitUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.reason)
    }
}

impl std::error::Error for RuntimeRiskLimitUpdateError {}

/// Position status projection for runtime control adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePositionStatus {
    Open,
    PartialFill,
    Closed,
}

/// Lightweight position projection for runtime control adapters.
#[derive(Debug, Clone)]
pub struct RuntimePosition {
    pub market_id: String,
    pub status: RuntimePositionStatus,
    pub entry_cost: Price,
    pub expected_profit: Price,
}

/// Read-only cluster projection for operator-facing adapters.
pub trait RuntimeClusterView: Send + Sync {
    /// Return all currently valid relation clusters.
    fn all_clusters(&self) -> Vec<Cluster>;
}

/// Port for mutable runtime state and operator controls.
pub trait RuntimeState: Send + Sync {
    /// Get current risk limits.
    fn risk_limits(&self) -> RuntimeRiskLimits;

    /// Update one risk limit.
    fn set_risk_limit(
        &self,
        kind: RuntimeRiskLimitKind,
        value: Decimal,
    ) -> Result<RuntimeRiskLimits, RuntimeRiskLimitUpdateError>;

    /// Whether circuit breaker is active.
    fn is_circuit_breaker_active(&self) -> bool;

    /// Optional reason for active circuit breaker.
    fn circuit_breaker_reason(&self) -> Option<String>;

    /// Activate circuit breaker with reason.
    fn activate_circuit_breaker(&self, reason: &str);

    /// Reset circuit breaker to active trading mode.
    fn reset_circuit_breaker(&self);

    /// Count open positions.
    fn open_position_count(&self) -> usize;

    /// Total open exposure.
    fn total_exposure(&self) -> Price;

    /// Exposure reserved for in-flight executions.
    fn pending_exposure(&self) -> Price;

    /// Number of in-flight executions.
    fn pending_execution_count(&self) -> usize;

    /// Active positions for operator-facing output.
    fn active_positions(&self) -> Vec<RuntimePosition>;
}
