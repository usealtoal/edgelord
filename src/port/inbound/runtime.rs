//! Runtime control port for operator-facing adapters.
//!
//! Defines interfaces for runtime state access and control, keeping control
//! surfaces (e.g., Telegram bot commands) decoupled from concrete application
//! state implementations.
//!
//! # Overview
//!
//! - [`RuntimeState`]: Mutable runtime state and operator controls
//! - [`RuntimeClusterView`]: Read-only access to discovered market clusters
//! - [`RuntimeRiskLimits`]: Current risk limit configuration

use rust_decimal::Decimal;

use crate::domain::cluster::Cluster;
use crate::domain::money::Price;

/// Snapshot of current risk limit settings.
///
/// Used by runtime control adapters to display and modify risk parameters.
#[derive(Debug, Clone)]
pub struct RuntimeRiskLimits {
    /// Maximum position size per market in USD.
    pub max_position_per_market: Decimal,

    /// Maximum total exposure across all open positions in USD.
    pub max_total_exposure: Decimal,

    /// Minimum profit threshold required to execute a trade in USD.
    pub min_profit_threshold: Decimal,

    /// Maximum acceptable slippage as a decimal (e.g., 0.02 = 2%).
    pub max_slippage: Decimal,
}

/// Enumeration of runtime-adjustable risk limit fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeRiskLimitKind {
    /// Maximum position size per market.
    MaxPositionPerMarket,

    /// Maximum total exposure across all positions.
    MaxTotalExposure,

    /// Minimum profit threshold for trade execution.
    MinProfitThreshold,

    /// Maximum acceptable slippage.
    MaxSlippage,
}

impl RuntimeRiskLimitKind {
    /// Return a stable string identifier for this risk limit.
    ///
    /// Used in logs, command output, and configuration keys.
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

/// Error returned when a runtime risk limit update is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeRiskLimitUpdateError {
    /// Description of why the update was rejected.
    reason: &'static str,
}

impl RuntimeRiskLimitUpdateError {
    /// Create a new update error with the specified reason.
    ///
    /// # Arguments
    ///
    /// * `reason` - Static description of the validation failure.
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

/// Status of a position for runtime display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePositionStatus {
    /// Position is fully open with all legs filled.
    Open,

    /// Position is partially filled (some legs executed).
    PartialFill,

    /// Position has been closed.
    Closed,
}

/// Lightweight position information for runtime control adapters.
///
/// Contains only the fields needed for operator-facing displays.
#[derive(Debug, Clone)]
pub struct RuntimePosition {
    /// Identifier of the market this position is in.
    pub market_id: String,

    /// Current status of the position.
    pub status: RuntimePositionStatus,

    /// Total cost to enter this position.
    pub entry_cost: Price,

    /// Expected profit if the position resolves favorably.
    pub expected_profit: Price,
}

/// Read-only view of discovered market clusters.
///
/// # Thread Safety
///
/// Implementations must be thread-safe (`Send + Sync`).
pub trait RuntimeClusterView: Send + Sync {
    /// Return all currently valid relation clusters.
    ///
    /// Clusters represent groups of markets with discovered logical relations.
    fn all_clusters(&self) -> Vec<Cluster>;
}

/// Mutable runtime state and operator control interface.
///
/// Provides access to runtime state and controls for operator-facing adapters
/// such as the Telegram bot.
///
/// # Thread Safety
///
/// Implementations must be thread-safe (`Send + Sync`) as this interface
/// may be accessed concurrently from multiple control surfaces.
pub trait RuntimeState: Send + Sync {
    /// Return the current risk limit settings.
    fn risk_limits(&self) -> RuntimeRiskLimits;

    /// Update a single risk limit parameter.
    ///
    /// # Arguments
    ///
    /// * `kind` - Which risk limit to update.
    /// * `value` - New value for the limit.
    ///
    /// # Errors
    ///
    /// Returns an error if the new value fails validation (e.g., negative amount).
    fn set_risk_limit(
        &self,
        kind: RuntimeRiskLimitKind,
        value: Decimal,
    ) -> Result<RuntimeRiskLimits, RuntimeRiskLimitUpdateError>;

    /// Return `true` if the circuit breaker is currently active.
    fn is_circuit_breaker_active(&self) -> bool;

    /// Return the reason for the active circuit breaker, if any.
    fn circuit_breaker_reason(&self) -> Option<String>;

    /// Activate the circuit breaker, halting all trading.
    ///
    /// # Arguments
    ///
    /// * `reason` - Human-readable description of why trading was halted.
    fn activate_circuit_breaker(&self, reason: &str);

    /// Reset the circuit breaker, resuming normal trading.
    fn reset_circuit_breaker(&self);

    /// Return the number of currently open positions.
    fn open_position_count(&self) -> usize;

    /// Return the total exposure across all open positions.
    fn total_exposure(&self) -> Price;

    /// Return the exposure reserved for in-flight (pending) executions.
    fn pending_exposure(&self) -> Price;

    /// Return the number of currently in-flight executions.
    fn pending_execution_count(&self) -> usize;

    /// Return all active positions for operator-facing display.
    fn active_positions(&self) -> Vec<RuntimePosition>;
}
