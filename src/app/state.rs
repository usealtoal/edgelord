//! Shared application state.

use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::RwLock;
use rust_decimal::Decimal;

use crate::domain::{PositionTracker, Price};

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
}

impl Default for RiskLimits {
    fn default() -> Self {
        Self {
            max_position_per_market: Decimal::from(1000),
            max_total_exposure: Decimal::from(10000),
            min_profit_threshold: Decimal::new(5, 2), // $0.05
            max_slippage: Decimal::new(2, 2),         // 2%
        }
    }
}

/// Shared application state accessible by all services.
pub struct AppState {
    /// Position tracker for all open/closed positions.
    positions: RwLock<PositionTracker>,
    /// Risk limits configuration.
    risk_limits: RiskLimits,
    /// Circuit breaker - when true, no new trades.
    circuit_breaker: AtomicBool,
    /// Reason for circuit breaker activation.
    circuit_breaker_reason: RwLock<Option<String>>,
}

impl AppState {
    /// Create new app state with given risk limits.
    #[must_use] 
    pub fn new(risk_limits: RiskLimits) -> Self {
        Self {
            positions: RwLock::new(PositionTracker::new()),
            risk_limits,
            circuit_breaker: AtomicBool::new(false),
            circuit_breaker_reason: RwLock::new(None),
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
    pub fn risk_limits(&self) -> &RiskLimits {
        &self.risk_limits
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
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(RiskLimits::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(state.circuit_breaker_reason(), Some("test reason".to_string()));

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
    fn test_total_exposure() {
        use crate::domain::{MarketId, Position, PositionLeg, PositionStatus, TokenId};
        use rust_decimal_macros::dec;

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
