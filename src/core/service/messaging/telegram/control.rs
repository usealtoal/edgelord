//! Telegram command execution against runtime app state.

use std::sync::Arc;

use chrono::Utc;

use crate::app::AppState;
use crate::core::domain::PositionStatus;

use super::command::{command_help, TelegramCommand};

/// Runtime command executor for Telegram control commands.
#[derive(Clone)]
pub struct TelegramControl {
    state: Arc<AppState>,
    started_at: chrono::DateTime<Utc>,
}

impl TelegramControl {
    #[must_use]
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            started_at: Utc::now(),
        }
    }

    /// Execute one parsed command and return response text.
    #[must_use]
    pub fn execute(&self, command: TelegramCommand) -> String {
        match command {
            TelegramCommand::Start | TelegramCommand::Help => command_help().to_string(),
            TelegramCommand::Status => self.status_text(),
            TelegramCommand::Health => self.health_text(),
            TelegramCommand::Positions => self.positions_text(),
            TelegramCommand::Pause => self.pause_text(),
            TelegramCommand::Resume => self.resume_text(),
            TelegramCommand::SetRisk { kind, value } => {
                match self.state.set_risk_limit(kind, value) {
                    Ok(limits) => format!(
                        "Updated {} to {}\n\
                    Current limits:\n\
                    min_profit: {}\n\
                    max_slippage: {}\n\
                    max_position: {}\n\
                    max_exposure: {}",
                        kind.as_str(),
                        value,
                        limits.min_profit_threshold,
                        limits.max_slippage,
                        limits.max_position_per_market,
                        limits.max_total_exposure
                    ),
                    Err(err) => format!("Cannot update {}: {}", kind.as_str(), err),
                }
            }
        }
    }

    fn status_text(&self) -> String {
        let limits = self.state.risk_limits();
        let open_positions = self.state.open_position_count();
        let exposure = self.state.total_exposure();
        let pending_exposure = self.state.pending_exposure();
        let pending_executions = self.state.pending_execution_count();
        let is_paused = self.state.is_circuit_breaker_active();

        let mode = if is_paused { "PAUSED" } else { "ACTIVE" };
        let breaker = if is_paused {
            self.state
                .circuit_breaker_reason()
                .unwrap_or_else(|| "unknown".to_string())
        } else {
            "inactive".to_string()
        };

        format!(
            "Edgelord status\n\
            mode: {mode}\n\
            circuit_breaker: {breaker}\n\
            uptime: {}\n\
            open_positions: {open_positions}\n\
            exposure: {exposure}\n\
            pending_exposure: {pending_exposure}\n\
            pending_executions: {pending_executions}\n\
            risk.min_profit: {}\n\
            risk.max_slippage: {}\n\
            risk.max_position: {}\n\
            risk.max_exposure: {}",
            format_uptime(self.started_at),
            limits.min_profit_threshold,
            limits.max_slippage,
            limits.max_position_per_market,
            limits.max_total_exposure
        )
    }

    fn health_text(&self) -> String {
        let limits = self.state.risk_limits();
        let exposure = self.state.total_exposure();
        let pending_exposure = self.state.pending_exposure();
        let exposure_ok = exposure + pending_exposure <= limits.max_total_exposure;
        let breaker_ok = !self.state.is_circuit_breaker_active();
        let slippage_ok = limits.max_slippage >= rust_decimal::Decimal::ZERO
            && limits.max_slippage <= rust_decimal::Decimal::ONE;

        let healthy = exposure_ok && breaker_ok && slippage_ok;
        let status = if healthy { "HEALTHY" } else { "DEGRADED" };

        let circuit_detail = if breaker_ok {
            "inactive".to_string()
        } else {
            self.state
                .circuit_breaker_reason()
                .unwrap_or_else(|| "active (no reason)".to_string())
        };

        format!(
            "Health: {status}\n\
            circuit_breaker: {circuit_detail}\n\
            exposure_ok: {exposure_ok} (current={}, pending={}, limit={})\n\
            slippage_ok: {slippage_ok} (max_slippage={})",
            exposure, pending_exposure, limits.max_total_exposure, limits.max_slippage
        )
    }

    fn positions_text(&self) -> String {
        let positions = self.state.positions();
        let active: Vec<_> = positions
            .all()
            .filter(|p| !p.status().is_closed())
            .take(10)
            .map(|p| {
                let status = match p.status() {
                    PositionStatus::Open => "open",
                    PositionStatus::PartialFill { .. } => "partial",
                    PositionStatus::Closed { .. } => "closed",
                };

                format!(
                    "{} market={} status={} entry_cost={} expected_profit={}",
                    p.id(),
                    p.market_id(),
                    status,
                    p.entry_cost(),
                    p.expected_profit()
                )
            })
            .collect();

        if active.is_empty() {
            return "No active positions".to_string();
        }

        let mut response = String::from("Active positions (max 10 shown)\n");
        response.push_str(&active.join("\n"));
        response
    }

    fn pause_text(&self) -> String {
        if self.state.is_circuit_breaker_active() {
            let reason = self
                .state
                .circuit_breaker_reason()
                .unwrap_or_else(|| "unknown".to_string());
            return format!("Trading already paused: {reason}");
        }

        self.state
            .activate_circuit_breaker("paused by telegram command");
        "Trading paused (circuit breaker activated)".to_string()
    }

    fn resume_text(&self) -> String {
        if !self.state.is_circuit_breaker_active() {
            return "Trading already active".to_string();
        }

        self.state.reset_circuit_breaker();
        "Trading resumed (circuit breaker reset)".to_string()
    }
}

fn format_uptime(started_at: chrono::DateTime<Utc>) -> String {
    let elapsed = Utc::now() - started_at;
    let total_seconds = elapsed.num_seconds().max(0);
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    use crate::app::{RiskLimitKind, RiskLimits};

    #[test]
    fn execute_pause_and_resume() {
        let state = Arc::new(AppState::default());
        let control = TelegramControl::new(Arc::clone(&state));

        let paused = control.execute(TelegramCommand::Pause);
        assert!(paused.contains("Trading paused"));
        assert!(state.is_circuit_breaker_active());

        let resumed = control.execute(TelegramCommand::Resume);
        assert!(resumed.contains("Trading resumed"));
        assert!(!state.is_circuit_breaker_active());
    }

    #[test]
    fn execute_set_risk_valid() {
        let state = Arc::new(AppState::default());
        let control = TelegramControl::new(Arc::clone(&state));

        let text = control.execute(TelegramCommand::SetRisk {
            kind: RiskLimitKind::MinProfitThreshold,
            value: dec!(0.4),
        });

        assert!(text.contains("Updated min_profit"));
        assert_eq!(state.risk_limits().min_profit_threshold, dec!(0.4));
    }

    #[test]
    fn execute_set_risk_invalid() {
        let state = Arc::new(AppState::new(RiskLimits::default()));
        let control = TelegramControl::new(state);

        let text = control.execute(TelegramCommand::SetRisk {
            kind: RiskLimitKind::MaxSlippage,
            value: dec!(2),
        });

        assert!(text.contains("Cannot update max_slippage"));
    }

    #[test]
    fn execute_positions_empty() {
        let state = Arc::new(AppState::default());
        let control = TelegramControl::new(state);

        assert_eq!(
            control.execute(TelegramCommand::Positions),
            "No active positions"
        );
    }
}
