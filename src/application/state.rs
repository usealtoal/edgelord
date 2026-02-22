//! Shared application state.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::{Mutex, RwLock};
use rust_decimal::Decimal;

use crate::application::cache::position::PositionTracker;
use crate::domain::{money::Price, position::PositionStatus};
use crate::port::inbound::runtime::{
    RuntimePosition, RuntimePositionStatus, RuntimeRiskLimitKind, RuntimeRiskLimitUpdateError,
    RuntimeRiskLimits, RuntimeState,
};

/// Risk limits configuration.
#[derive(Debug, Clone)]
pub struct RiskLimits {
    /// Maximum position size per market in dollars.
    pub max_position_per_market: Decimal,
    /// Maximum total exposure across all positions.
    pub max_total_exposure: Decimal,
    /// Minimum profit threshold to execute.
    pub min_profit_threshold: Decimal,
    /// Maximum slippage tolerance (e.g., 0.02 = 2%).
    pub max_slippage: Decimal,
    /// Execution timeout in seconds.
    pub execution_timeout_secs: u64,
}

impl Default for RiskLimits {
    fn default() -> Self {
        Self {
            max_position_per_market: Decimal::from(1000),
            max_total_exposure: Decimal::from(10000),
            min_profit_threshold: Decimal::new(5, 2), // $0.05
            max_slippage: Decimal::new(2, 2),         // 2%
            execution_timeout_secs: 30,
        }
    }
}

/// Risk limit fields that may be changed at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLimitKind {
    MaxPositionPerMarket,
    MaxTotalExposure,
    MinProfitThreshold,
    MaxSlippage,
}

impl RiskLimitKind {
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

/// Error returned when a runtime risk limit update is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiskLimitUpdateError {
    reason: &'static str,
}

impl RiskLimitUpdateError {
    #[must_use]
    pub const fn new(reason: &'static str) -> Self {
        Self { reason }
    }

    #[must_use]
    pub const fn reason(&self) -> &'static str {
        self.reason
    }
}

impl std::fmt::Display for RiskLimitUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.reason)
    }
}

impl std::error::Error for RiskLimitUpdateError {}

fn app_limits_to_runtime(limits: RiskLimits) -> RuntimeRiskLimits {
    RuntimeRiskLimits {
        max_position_per_market: limits.max_position_per_market,
        max_total_exposure: limits.max_total_exposure,
        min_profit_threshold: limits.min_profit_threshold,
        max_slippage: limits.max_slippage,
    }
}

fn runtime_kind_to_app(kind: RuntimeRiskLimitKind) -> RiskLimitKind {
    match kind {
        RuntimeRiskLimitKind::MaxPositionPerMarket => RiskLimitKind::MaxPositionPerMarket,
        RuntimeRiskLimitKind::MaxTotalExposure => RiskLimitKind::MaxTotalExposure,
        RuntimeRiskLimitKind::MinProfitThreshold => RiskLimitKind::MinProfitThreshold,
        RuntimeRiskLimitKind::MaxSlippage => RiskLimitKind::MaxSlippage,
    }
}

/// Shared application state accessible by all services.
pub struct AppState {
    /// Position tracker for all open/closed positions.
    positions: RwLock<PositionTracker>,
    /// Risk limits configuration.
    risk_limits: RwLock<RiskLimits>,
    /// Circuit breaker - when true, no new trades.
    circuit_breaker: AtomicBool,
    /// Reason for circuit breaker activation.
    circuit_breaker_reason: RwLock<Option<String>>,
    /// Markets with in-flight executions (prevents duplicate trades).
    pending_executions: Mutex<HashSet<String>>,
    /// Pending exposure from approved opportunities not yet executed.
    pending_exposure: Mutex<Decimal>,
}

impl AppState {
    /// Create new app state with given risk limits.
    #[must_use]
    pub fn new(risk_limits: RiskLimits) -> Self {
        Self {
            positions: RwLock::new(PositionTracker::new()),
            risk_limits: RwLock::new(risk_limits),
            circuit_breaker: AtomicBool::new(false),
            circuit_breaker_reason: RwLock::new(None),
            pending_executions: Mutex::new(HashSet::new()),
            pending_exposure: Mutex::new(Decimal::ZERO),
        }
    }

    /// Get read access to positions.
    pub fn positions(&self) -> parking_lot::RwLockReadGuard<'_, PositionTracker> {
        self.positions.read()
    }

    /// Get write access to positions.
    pub fn positions_mut(&self) -> parking_lot::RwLockWriteGuard<'_, PositionTracker> {
        self.positions.write()
    }

    /// Get risk limits.
    pub fn risk_limits(&self) -> RiskLimits {
        self.risk_limits.read().clone()
    }

    /// Set one risk limit at runtime with validation.
    ///
    /// Returns a full snapshot of limits after the update.
    pub fn set_risk_limit(
        &self,
        kind: RiskLimitKind,
        value: Decimal,
    ) -> Result<RiskLimits, RiskLimitUpdateError> {
        match kind {
            RiskLimitKind::MaxPositionPerMarket | RiskLimitKind::MaxTotalExposure => {
                if value <= Decimal::ZERO {
                    return Err(RiskLimitUpdateError::new("value must be greater than 0"));
                }
            }
            RiskLimitKind::MinProfitThreshold => {
                if value < Decimal::ZERO {
                    return Err(RiskLimitUpdateError::new("value must be 0 or greater"));
                }
            }
            RiskLimitKind::MaxSlippage => {
                if value < Decimal::ZERO || value > Decimal::ONE {
                    return Err(RiskLimitUpdateError::new("value must be between 0 and 1"));
                }
            }
        }

        let mut limits = self.risk_limits.write();
        match kind {
            RiskLimitKind::MaxPositionPerMarket => limits.max_position_per_market = value,
            RiskLimitKind::MaxTotalExposure => limits.max_total_exposure = value,
            RiskLimitKind::MinProfitThreshold => limits.min_profit_threshold = value,
            RiskLimitKind::MaxSlippage => limits.max_slippage = value,
        }

        Ok(limits.clone())
    }

    /// Check if circuit breaker is active.
    pub fn is_circuit_breaker_active(&self) -> bool {
        self.circuit_breaker.load(Ordering::SeqCst)
    }

    /// Activate circuit breaker with reason.
    pub fn activate_circuit_breaker(&self, reason: impl Into<String>) {
        self.circuit_breaker.store(true, Ordering::SeqCst);
        *self.circuit_breaker_reason.write() = Some(reason.into());
    }

    /// Reset circuit breaker.
    pub fn reset_circuit_breaker(&self) {
        self.circuit_breaker.store(false, Ordering::SeqCst);
        *self.circuit_breaker_reason.write() = None;
    }

    /// Get circuit breaker reason if active.
    pub fn circuit_breaker_reason(&self) -> Option<String> {
        self.circuit_breaker_reason.read().clone()
    }

    /// Get current total exposure.
    pub fn total_exposure(&self) -> Price {
        self.positions.read().total_exposure()
    }

    /// Count open positions.
    pub fn open_position_count(&self) -> usize {
        self.positions.read().open_count()
    }

    /// Try to acquire execution lock for a market.
    /// Returns `true` if lock acquired, `false` if already locked.
    pub fn try_lock_execution(&self, market_id: &str) -> bool {
        self.pending_executions.lock().insert(market_id.to_string())
    }

    /// Release execution lock for a market.
    pub fn release_execution(&self, market_id: &str) {
        self.pending_executions.lock().remove(market_id);
    }

    /// Count markets with in-flight executions.
    pub fn pending_execution_count(&self) -> usize {
        self.pending_executions.lock().len()
    }

    /// Get current pending exposure.
    pub fn pending_exposure(&self) -> Price {
        *self.pending_exposure.lock()
    }

    /// Try to reserve exposure atomically.
    /// Returns `true` if reservation succeeded, `false` if it would exceed limit.
    pub fn try_reserve_exposure(&self, amount: Price) -> bool {
        let positions = self.positions.read();
        let mut pending = self.pending_exposure.lock();
        let current = positions.total_exposure();
        let limit = self.risk_limits.read().max_total_exposure;

        if current + *pending + amount > limit {
            return false;
        }

        *pending += amount;
        true
    }

    /// Release reserved exposure.
    pub fn release_exposure(&self, amount: Price) {
        let mut pending = self.pending_exposure.lock();
        *pending -= amount;
        // Ensure we don't go negative (shouldn't happen, but safety check)
        if *pending < Decimal::ZERO {
            *pending = Decimal::ZERO;
        }
    }
}

impl RuntimeState for AppState {
    fn risk_limits(&self) -> RuntimeRiskLimits {
        app_limits_to_runtime(AppState::risk_limits(self))
    }

    fn set_risk_limit(
        &self,
        kind: RuntimeRiskLimitKind,
        value: Decimal,
    ) -> Result<RuntimeRiskLimits, RuntimeRiskLimitUpdateError> {
        AppState::set_risk_limit(self, runtime_kind_to_app(kind), value)
            .map(app_limits_to_runtime)
            .map_err(|err| RuntimeRiskLimitUpdateError::new(err.reason()))
    }

    fn is_circuit_breaker_active(&self) -> bool {
        AppState::is_circuit_breaker_active(self)
    }

    fn circuit_breaker_reason(&self) -> Option<String> {
        AppState::circuit_breaker_reason(self)
    }

    fn activate_circuit_breaker(&self, reason: &str) {
        AppState::activate_circuit_breaker(self, reason);
    }

    fn reset_circuit_breaker(&self) {
        AppState::reset_circuit_breaker(self);
    }

    fn open_position_count(&self) -> usize {
        AppState::open_position_count(self)
    }

    fn total_exposure(&self) -> Price {
        AppState::total_exposure(self)
    }

    fn pending_exposure(&self) -> Price {
        AppState::pending_exposure(self)
    }

    fn pending_execution_count(&self) -> usize {
        AppState::pending_execution_count(self)
    }

    fn active_positions(&self) -> Vec<RuntimePosition> {
        AppState::positions(self)
            .all()
            .filter(|p| !p.status().is_closed())
            .map(|p| RuntimePosition {
                market_id: p.market_id().as_str().to_string(),
                status: match p.status() {
                    PositionStatus::Open => RuntimePositionStatus::Open,
                    PositionStatus::PartialFill { .. } => RuntimePositionStatus::PartialFill,
                    PositionStatus::Closed { .. } => RuntimePositionStatus::Closed,
                },
                entry_cost: p.entry_cost(),
                expected_profit: p.expected_profit(),
            })
            .collect()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(RiskLimits::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_app_state_default() {
        let state = AppState::default();
        assert!(!state.is_circuit_breaker_active());
        assert!(state.circuit_breaker_reason().is_none());
    }

    #[test]
    fn test_circuit_breaker() {
        let state = AppState::default();

        state.activate_circuit_breaker("test reason");
        assert!(state.is_circuit_breaker_active());
        assert_eq!(
            state.circuit_breaker_reason(),
            Some("test reason".to_string())
        );

        state.reset_circuit_breaker();
        assert!(!state.is_circuit_breaker_active());
        assert!(state.circuit_breaker_reason().is_none());
    }

    #[test]
    fn test_risk_limits_default() {
        let limits = RiskLimits::default();
        assert_eq!(limits.max_position_per_market, Decimal::from(1000));
        assert_eq!(limits.max_total_exposure, Decimal::from(10000));
    }

    #[test]
    fn test_set_risk_limit_updates_value() {
        let state = AppState::default();

        let updated = state
            .set_risk_limit(RiskLimitKind::MinProfitThreshold, dec!(0.25))
            .unwrap();

        assert_eq!(updated.min_profit_threshold, dec!(0.25));
        assert_eq!(state.risk_limits().min_profit_threshold, dec!(0.25));
    }

    #[test]
    fn test_set_risk_limit_rejects_invalid() {
        let state = AppState::default();

        let err = state
            .set_risk_limit(RiskLimitKind::MaxSlippage, dec!(1.5))
            .unwrap_err();

        assert_eq!(err.reason(), "value must be between 0 and 1");
        assert_eq!(state.risk_limits().max_slippage, Decimal::new(2, 2));
    }

    #[test]
    fn test_execution_locking() {
        let state = AppState::default();

        // First lock should succeed
        assert!(state.try_lock_execution("market-1"));

        // Second lock on same market should fail
        assert!(!state.try_lock_execution("market-1"));

        // Different market should succeed
        assert!(state.try_lock_execution("market-2"));

        // After release, can lock again
        state.release_execution("market-1");
        assert!(state.try_lock_execution("market-1"));
    }

    #[test]
    fn test_total_exposure() {
        use crate::domain::{
            id::MarketId, id::TokenId, position::Position, position::PositionLeg,
            position::PositionStatus,
        };

        let state = AppState::default();

        // Initially zero
        assert_eq!(state.total_exposure(), Decimal::ZERO);

        // Add a position
        {
            let mut positions = state.positions_mut();
            let position = Position::new(
                positions.next_id(),
                MarketId::new("test"),
                vec![
                    PositionLeg::new(TokenId::new("yes"), dec!(100), dec!(0.45)),
                    PositionLeg::new(TokenId::new("no"), dec!(100), dec!(0.45)),
                ],
                dec!(90), // entry cost
                dec!(100),
                chrono::Utc::now(),
                PositionStatus::Open,
            );
            positions.add(position);
        }

        // Now has exposure
        assert_eq!(state.total_exposure(), dec!(90));
    }
}
