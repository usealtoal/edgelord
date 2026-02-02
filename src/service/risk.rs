//! Risk management service.
//!
//! Provides pre-execution checks for position limits, exposure caps,
//! and circuit breaker status.

use std::sync::Arc;

use rust_decimal::Decimal;
use tracing::{info, warn};

use crate::app::AppState;
use crate::domain::Opportunity;
use crate::error::RiskError;

/// Result of a risk check.
#[derive(Debug, Clone)]
pub enum RiskCheckResult {
    /// Trade is allowed to proceed.
    Approved,
    /// Trade is rejected with reason.
    Rejected(RiskError),
}

impl RiskCheckResult {
    /// Check if approved.
    pub fn is_approved(&self) -> bool {
        matches!(self, RiskCheckResult::Approved)
    }

    /// Get rejection error if rejected.
    pub fn rejection_error(&self) -> Option<&RiskError> {
        match self {
            RiskCheckResult::Rejected(e) => Some(e),
            RiskCheckResult::Approved => None,
        }
    }
}

/// Risk manager that validates trades before execution.
pub struct RiskManager {
    state: Arc<AppState>,
}

impl RiskManager {
    /// Create a new risk manager with shared state.
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    /// Check if an opportunity passes all risk checks.
    pub fn check(&self, opportunity: &Opportunity) -> RiskCheckResult {
        // Check circuit breaker first
        if let Err(e) = self.check_circuit_breaker() {
            return RiskCheckResult::Rejected(e);
        }

        // Check profit threshold
        if let Err(e) = self.check_profit_threshold(opportunity) {
            return RiskCheckResult::Rejected(e);
        }

        // Check exposure limits
        if let Err(e) = self.check_exposure_limit(opportunity) {
            return RiskCheckResult::Rejected(e);
        }

        // Check position limit for this market
        if let Err(e) = self.check_position_limit(opportunity) {
            return RiskCheckResult::Rejected(e);
        }

        RiskCheckResult::Approved
    }

    /// Check if circuit breaker is active.
    fn check_circuit_breaker(&self) -> Result<(), RiskError> {
        if self.state.is_circuit_breaker_active() {
            let reason = self
                .state
                .circuit_breaker_reason()
                .unwrap_or_else(|| "unknown".to_string());
            warn!(reason = %reason, "Circuit breaker active");
            return Err(RiskError::CircuitBreakerActive { reason });
        }
        Ok(())
    }

    /// Check if expected profit meets threshold.
    fn check_profit_threshold(&self, opportunity: &Opportunity) -> Result<(), RiskError> {
        let threshold = self.state.risk_limits().min_profit_threshold;
        let expected = opportunity.expected_profit();

        if expected < threshold {
            return Err(RiskError::ProfitBelowThreshold { expected, threshold });
        }
        Ok(())
    }

    /// Check if total exposure would exceed limit.
    fn check_exposure_limit(&self, opportunity: &Opportunity) -> Result<(), RiskError> {
        let current = self.state.total_exposure();
        let additional = opportunity.total_cost() * opportunity.volume();
        let limit = self.state.risk_limits().max_total_exposure;

        if current + additional > limit {
            warn!(
                current = %current,
                additional = %additional,
                limit = %limit,
                "Exposure limit would be exceeded"
            );
            return Err(RiskError::ExposureLimitExceeded {
                current,
                additional,
                limit,
            });
        }
        Ok(())
    }

    /// Check if position in this market would exceed limit.
    fn check_position_limit(&self, opportunity: &Opportunity) -> Result<(), RiskError> {
        let market_id = opportunity.market_id();
        let limit = self.state.risk_limits().max_position_per_market;

        // Calculate current position in this market
        let current = self
            .state
            .positions()
            .open_positions()
            .filter(|p| p.market_id() == market_id)
            .map(|p| p.entry_cost())
            .sum::<Decimal>();

        let additional = opportunity.total_cost() * opportunity.volume();

        if current + additional > limit {
            warn!(
                market_id = %market_id,
                current = %current,
                additional = %additional,
                limit = %limit,
                "Position limit would be exceeded"
            );
            return Err(RiskError::PositionLimitExceeded {
                market_id: market_id.to_string(),
                current,
                limit,
            });
        }
        Ok(())
    }

    /// Record a successful execution (updates state).
    pub fn record_execution(&self, opportunity: &Opportunity) {
        info!(
            market_id = %opportunity.market_id(),
            volume = %opportunity.volume(),
            profit = %opportunity.expected_profit(),
            "Execution recorded"
        );
        // Position is added by executor, we just log here
    }

    /// Trigger circuit breaker.
    pub fn trigger_circuit_breaker(&self, reason: impl Into<String>) {
        let reason = reason.into();
        warn!(reason = %reason, "Triggering circuit breaker");
        self.state.activate_circuit_breaker(reason);
    }

    /// Reset circuit breaker.
    pub fn reset_circuit_breaker(&self) {
        info!("Resetting circuit breaker");
        self.state.reset_circuit_breaker();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::RiskLimits;
    use crate::domain::{MarketId, TokenId};
    use rust_decimal_macros::dec;

    fn make_opportunity(volume: Decimal, yes_ask: Decimal, no_ask: Decimal) -> Opportunity {
        Opportunity::builder()
            .market_id(MarketId::from("test-market"))
            .question("Test?")
            .yes_token(TokenId::from("yes"), yes_ask)
            .no_token(TokenId::from("no"), no_ask)
            .volume(volume)
            .build()
            .unwrap()
    }

    #[test]
    fn test_check_approved() {
        let state = Arc::new(AppState::default());
        let risk = RiskManager::new(state);

        // Good opportunity: 10% edge, $10 volume = $1 profit
        let opp = make_opportunity(dec!(10), dec!(0.45), dec!(0.45));
        let result = risk.check(&opp);

        assert!(result.is_approved());
    }

    #[test]
    fn test_check_circuit_breaker() {
        let state = Arc::new(AppState::default());
        state.activate_circuit_breaker("test");
        let risk = RiskManager::new(state);

        let opp = make_opportunity(dec!(10), dec!(0.45), dec!(0.45));
        let result = risk.check(&opp);

        assert!(!result.is_approved());
        assert!(matches!(
            result.rejection_error(),
            Some(RiskError::CircuitBreakerActive { .. })
        ));
    }

    #[test]
    fn test_check_profit_below_threshold() {
        let limits = RiskLimits {
            min_profit_threshold: dec!(1.0), // Require $1 minimum
            ..Default::default()
        };
        let state = Arc::new(AppState::new(limits));
        let risk = RiskManager::new(state);

        // Only $0.10 edge * $1 volume = $0.10 profit (below $1 threshold)
        let opp = make_opportunity(dec!(1), dec!(0.45), dec!(0.45));
        let result = risk.check(&opp);

        assert!(!result.is_approved());
        assert!(matches!(
            result.rejection_error(),
            Some(RiskError::ProfitBelowThreshold { .. })
        ));
    }

    #[test]
    fn test_check_exposure_limit() {
        let limits = RiskLimits {
            max_total_exposure: dec!(100), // Only $100 max
            min_profit_threshold: dec!(0),
            ..Default::default()
        };
        let state = Arc::new(AppState::new(limits));
        let risk = RiskManager::new(state);

        // $0.90 cost * $200 volume = $180 exposure (over $100)
        let opp = make_opportunity(dec!(200), dec!(0.45), dec!(0.45));
        let result = risk.check(&opp);

        assert!(!result.is_approved());
        assert!(matches!(
            result.rejection_error(),
            Some(RiskError::ExposureLimitExceeded { .. })
        ));
    }

    #[test]
    fn test_trigger_and_reset_circuit_breaker() {
        let state = Arc::new(AppState::default());
        let risk = RiskManager::new(state.clone());

        assert!(!state.is_circuit_breaker_active());

        risk.trigger_circuit_breaker("test reason");
        assert!(state.is_circuit_breaker_active());

        risk.reset_circuit_breaker();
        assert!(!state.is_circuit_breaker_active());
    }
}
