//! Comprehensive tests for TelegramControl and related components.

use super::super::command::TelegramCommand;
use super::*;
use parking_lot::RwLock;
use rust_decimal_macros::dec;

use crate::adapter::outbound::sqlite::database;
use crate::adapter::outbound::sqlite::recorder;
use crate::port::inbound::runtime::{
    RuntimePosition, RuntimePositionStatus, RuntimeRiskLimitKind, RuntimeRiskLimitUpdateError,
    RuntimeRiskLimits, RuntimeState,
};

// =============================================================================
// Mock Runtime State
// =============================================================================

#[derive(Debug, Default)]
struct TestPositionStore {
    positions: Vec<crate::domain::position::Position>,
    next_id: u64,
}

impl TestPositionStore {
    fn next_id(&mut self) -> crate::domain::id::PositionId {
        let id = crate::domain::id::PositionId::new(self.next_id + 1);
        self.next_id += 1;
        id
    }

    fn add(&mut self, position: crate::domain::position::Position) {
        self.positions.push(position);
    }

    fn all(&self) -> impl Iterator<Item = &crate::domain::position::Position> {
        self.positions.iter()
    }
}

#[derive(Debug)]
struct MockRuntimeState {
    limits: RwLock<RuntimeRiskLimits>,
    breaker_reason: RwLock<Option<String>>,
    positions: RwLock<TestPositionStore>,
    pending_exposure: RwLock<rust_decimal::Decimal>,
    pending_executions: RwLock<usize>,
}

impl Default for MockRuntimeState {
    fn default() -> Self {
        Self {
            limits: RwLock::new(RuntimeRiskLimits {
                max_position_per_market: dec!(100),
                max_total_exposure: dec!(1000),
                min_profit_threshold: dec!(0.2),
                max_slippage: dec!(0.05),
            }),
            breaker_reason: RwLock::new(None),
            positions: RwLock::new(TestPositionStore::default()),
            pending_exposure: RwLock::new(dec!(0)),
            pending_executions: RwLock::new(0),
        }
    }
}

impl MockRuntimeState {
    fn positions_mut(&self) -> parking_lot::RwLockWriteGuard<'_, TestPositionStore> {
        self.positions.write()
    }

    fn with_limits(limits: RuntimeRiskLimits) -> Self {
        Self {
            limits: RwLock::new(limits),
            ..Default::default()
        }
    }

    fn set_pending(&self, exposure: rust_decimal::Decimal, count: usize) {
        *self.pending_exposure.write() = exposure;
        *self.pending_executions.write() = count;
    }
}

impl RuntimeState for MockRuntimeState {
    fn risk_limits(&self) -> RuntimeRiskLimits {
        self.limits.read().clone()
    }

    fn set_risk_limit(
        &self,
        kind: RuntimeRiskLimitKind,
        value: rust_decimal::Decimal,
    ) -> Result<RuntimeRiskLimits, RuntimeRiskLimitUpdateError> {
        match kind {
            RuntimeRiskLimitKind::MaxPositionPerMarket | RuntimeRiskLimitKind::MaxTotalExposure
                if value <= rust_decimal::Decimal::ZERO =>
            {
                return Err(RuntimeRiskLimitUpdateError::new("must be greater than 0"));
            }
            RuntimeRiskLimitKind::MinProfitThreshold if value < rust_decimal::Decimal::ZERO => {
                return Err(RuntimeRiskLimitUpdateError::new(
                    "min_profit must be 0 or greater",
                ));
            }
            RuntimeRiskLimitKind::MaxSlippage
                if value < rust_decimal::Decimal::ZERO || value > rust_decimal::Decimal::ONE =>
            {
                return Err(RuntimeRiskLimitUpdateError::new(
                    "max_slippage must be between 0 and 1",
                ));
            }
            _ => {}
        }

        let mut limits = self.limits.write();
        match kind {
            RuntimeRiskLimitKind::MaxPositionPerMarket => limits.max_position_per_market = value,
            RuntimeRiskLimitKind::MaxTotalExposure => limits.max_total_exposure = value,
            RuntimeRiskLimitKind::MinProfitThreshold => limits.min_profit_threshold = value,
            RuntimeRiskLimitKind::MaxSlippage => limits.max_slippage = value,
        }
        Ok(limits.clone())
    }

    fn is_circuit_breaker_active(&self) -> bool {
        self.breaker_reason.read().is_some()
    }

    fn circuit_breaker_reason(&self) -> Option<String> {
        self.breaker_reason.read().clone()
    }

    fn activate_circuit_breaker(&self, reason: &str) {
        *self.breaker_reason.write() = Some(reason.to_string());
    }

    fn reset_circuit_breaker(&self) {
        *self.breaker_reason.write() = None;
    }

    fn open_position_count(&self) -> usize {
        self.positions
            .read()
            .all()
            .filter(|position| !position.status().is_closed())
            .count()
    }

    fn total_exposure(&self) -> crate::domain::money::Price {
        self.positions
            .read()
            .all()
            .filter(|position| position.status().is_open())
            .map(crate::domain::position::Position::entry_cost)
            .sum()
    }

    fn pending_exposure(&self) -> crate::domain::money::Price {
        *self.pending_exposure.read()
    }

    fn pending_execution_count(&self) -> usize {
        *self.pending_executions.read()
    }

    fn active_positions(&self) -> Vec<RuntimePosition> {
        self.positions
            .read()
            .all()
            .filter(|position| !position.status().is_closed())
            .map(|position| {
                let status = match position.status() {
                    crate::domain::position::PositionStatus::Open => RuntimePositionStatus::Open,
                    crate::domain::position::PositionStatus::PartialFill { .. } => {
                        RuntimePositionStatus::PartialFill
                    }
                    crate::domain::position::PositionStatus::Closed { .. } => {
                        RuntimePositionStatus::Closed
                    }
                };

                RuntimePosition {
                    market_id: position.market_id().as_str().to_string(),
                    status,
                    entry_cost: position.entry_cost(),
                    expected_profit: position.expected_profit(),
                }
            })
            .collect()
    }
}

fn as_runtime(state: Arc<MockRuntimeState>) -> Arc<dyn RuntimeState> {
    state
}

#[test]
fn execute_pause_and_resume() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(Arc::clone(&state)));

    let paused = control.execute(TelegramCommand::Pause);
    assert!(paused.contains("paused"));
    assert!(state.is_circuit_breaker_active());

    let resumed = control.execute(TelegramCommand::Resume);
    assert!(resumed.contains("resumed"));
    assert!(!state.is_circuit_breaker_active());
}

#[test]
fn execute_set_risk_valid() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(Arc::clone(&state)));

    let text = control.execute(TelegramCommand::SetRisk {
        kind: RuntimeRiskLimitKind::MinProfitThreshold,
        value: dec!(0.4),
    });

    assert!(text.contains("Updated min_profit"));
    assert_eq!(state.risk_limits().min_profit_threshold, dec!(0.4));
}

#[test]
fn execute_set_risk_invalid() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(state));

    let text = control.execute(TelegramCommand::SetRisk {
        kind: RuntimeRiskLimitKind::MaxSlippage,
        value: dec!(2),
    });

    assert!(text.contains("Error"));
    assert!(text.contains("max_slippage"));
}

#[test]
fn execute_positions_empty() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(state));

    let text = control.execute(TelegramCommand::Positions);
    assert!(text.contains("No active positions"));
}

#[test]
fn execute_version() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(state));

    let text = control.execute(TelegramCommand::Version);
    assert!(text.contains("Version"));
    assert!(text.contains("v0."));
}

#[test]
fn execute_stats_without_recorder() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(state));

    let text = control.execute(TelegramCommand::Stats);
    assert!(text.contains("not available"));
}

#[test]
fn execute_pool_without_runtime_stats() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(state));

    let text = control.execute(TelegramCommand::Pool);
    assert!(text.contains("not available"));
}

#[test]
fn runtime_stats_update_and_read() {
    let stats = RuntimeStats::new();

    stats.update_market_counts(10, 20);
    assert_eq!(stats.market_count(), 10);
    assert_eq!(stats.token_count(), 20);

    stats.update_pool_stats(PoolStats {
        active_connections: 3,
        total_rotations: 5,
        total_restarts: 1,
        events_dropped: 0,
    });

    let pool = stats.pool_stats().unwrap();
    assert_eq!(pool.active_connections, 3);
    assert_eq!(pool.total_rotations, 5);
}

#[test]
fn execute_stats_with_recorder() {
    // Create in-memory database for test.
    let pool = database::connection::create_pool("sqlite://:memory:").expect("create pool");
    database::connection::run_migrations(&pool).expect("run migrations");
    let recorder = recorder::create_recorder(pool);

    let state = Arc::new(MockRuntimeState::default());
    let runtime = Arc::new(RuntimeStats::new());
    let control = TelegramControl::with_config(as_runtime(state), recorder, runtime, 10);

    let text = control.execute(TelegramCommand::Stats);
    assert!(text.contains("Today's Statistics"));
    assert!(text.contains("Opportunities:"));
    assert!(text.contains("P&L"));
}

#[test]
fn execute_pool_with_stats() {
    let state = Arc::new(MockRuntimeState::default());
    let pool = database::connection::create_pool("sqlite://:memory:").expect("create pool");
    database::connection::run_migrations(&pool).expect("run migrations");
    let recorder = recorder::create_recorder(pool);
    let runtime = Arc::new(RuntimeStats::new());

    // Update pool stats.
    runtime.update_pool_stats(PoolStats {
        active_connections: 5,
        total_rotations: 10,
        total_restarts: 2,
        events_dropped: 100,
    });

    let control = TelegramControl::with_config(as_runtime(state), recorder, runtime, 10);

    let text = control.execute(TelegramCommand::Pool);
    assert!(text.contains("Connection Pool"));
    assert!(text.contains("Active Connections: 5"));
    assert!(text.contains("TTL Rotations: 10"));
    assert!(text.contains("Events Dropped: 100"));
}

#[test]
fn execute_markets_with_stats() {
    let state = Arc::new(MockRuntimeState::default());
    let pool = database::connection::create_pool("sqlite://:memory:").expect("create pool");
    database::connection::run_migrations(&pool).expect("run migrations");
    let recorder = recorder::create_recorder(pool);
    let runtime = Arc::new(RuntimeStats::new());

    // Update market counts.
    runtime.update_market_counts(42, 84);

    let control = TelegramControl::with_config(as_runtime(state), recorder, runtime, 10);

    let text = control.execute(TelegramCommand::Markets);
    assert!(text.contains("Subscribed Markets"));
    assert!(text.contains("Markets: 42"));
    assert!(text.contains("Tokens: 84"));
}

#[test]
fn execute_markets_without_runtime_stats() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(state));

    let text = control.execute(TelegramCommand::Markets);
    assert!(text.contains("not available"));
}

#[test]
fn execute_markets_with_zero_counts() {
    let state = Arc::new(MockRuntimeState::default());
    let pool = database::connection::create_pool("sqlite://:memory:").expect("create pool");
    database::connection::run_migrations(&pool).expect("run migrations");
    let recorder = recorder::create_recorder(pool);
    let runtime = Arc::new(RuntimeStats::new());
    // Don't update counts - they default to 0.

    let control = TelegramControl::with_config(as_runtime(state), recorder, runtime, 10);

    let text = control.execute(TelegramCommand::Markets);
    assert!(text.contains("No markets subscribed"));
}

#[test]
fn execute_pool_not_initialized() {
    let state = Arc::new(MockRuntimeState::default());
    let pool = database::connection::create_pool("sqlite://:memory:").expect("create pool");
    database::connection::run_migrations(&pool).expect("run migrations");
    let recorder = recorder::create_recorder(pool);
    let runtime = Arc::new(RuntimeStats::new());
    // Don't update pool stats.

    let control = TelegramControl::with_config(as_runtime(state), recorder, runtime, 10);

    let text = control.execute(TelegramCommand::Pool);
    assert!(text.contains("Pool not initialized"));
}

#[test]
fn execute_help_command() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(state));

    let text = control.execute(TelegramCommand::Help);
    assert!(text.contains("/status"));
    assert!(text.contains("/health"));
}

#[test]
fn execute_start_command() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(state));

    let text = control.execute(TelegramCommand::Start);
    assert!(text.contains("/status"));
    assert!(text.contains("/positions"));
    assert!(text.contains("Commands"));
}

#[test]
fn execute_status_command() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(state));

    let text = control.execute(TelegramCommand::Status);
    assert!(text.contains("Status"));
    assert!(text.contains("Mode:"));
    assert!(text.contains("Risk Limits"));
    assert!(text.contains("Portfolio"));
}

#[test]
fn execute_health_command() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(state));

    let text = control.execute(TelegramCommand::Health);
    assert!(text.contains("Health Check:"));
    assert!(text.contains("Circuit Breaker:"));
    assert!(text.contains("Exposure:"));
}

#[test]
fn pause_when_already_paused() {
    let state = Arc::new(MockRuntimeState::default());
    state.activate_circuit_breaker("manual pause");
    let control = TelegramControl::new(as_runtime(Arc::clone(&state)));

    let text = control.execute(TelegramCommand::Pause);
    assert!(text.contains("Already paused"));
}

#[test]
fn resume_when_not_paused() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(state));

    let text = control.execute(TelegramCommand::Resume);
    assert!(text.contains("Trading already active"));
}

#[test]
fn positions_with_data() {
    use crate::domain::{
        id::MarketId, id::TokenId, position::Position, position::PositionLeg,
        position::PositionStatus,
    };

    let state = Arc::new(MockRuntimeState::default());

    // Add a position.
    {
        let mut positions = state.positions_mut();
        let legs = vec![
            PositionLeg::new(TokenId::from("yes-1"), dec!(100), dec!(0.40)),
            PositionLeg::new(TokenId::from("no-1"), dec!(100), dec!(0.50)),
        ];
        let position = Position::new(
            positions.next_id(),
            MarketId::from("test-market-12345"),
            legs,
            dec!(90),
            dec!(100),
            chrono::Utc::now(),
            PositionStatus::Open,
        );
        positions.add(position);
    }

    let control = TelegramControl::new(as_runtime(Arc::clone(&state)));
    let text = control.execute(TelegramCommand::Positions);

    assert!(text.contains("Active Positions (1)"));
    assert!(text.contains("test-market-"));
    assert!(text.contains("open"));
}

#[test]
fn format_uptime_test() {
    use chrono::{Duration, Utc};

    let now = Utc::now();
    let started = now - Duration::hours(2) - Duration::minutes(30) - Duration::seconds(45);

    let uptime = format_uptime(started);
    assert!(uptime.contains("02:30:45") || uptime.contains("02:30:46")); // Allow for timing
}

#[test]
fn runtime_stats_pool_stats_none_initially() {
    let stats = RuntimeStats::new();
    assert!(stats.pool_stats().is_none());
}

#[test]
fn status_shows_paused_when_circuit_breaker_active() {
    let state = Arc::new(MockRuntimeState::default());
    state.activate_circuit_breaker("test reason");
    let control = TelegramControl::new(as_runtime(Arc::clone(&state)));

    let text = control.execute(TelegramCommand::Status);
    assert!(text.contains("PAUSED"));
    assert!(text.contains("test reason"));
}

#[test]
fn health_shows_degraded_when_circuit_breaker_active() {
    let state = Arc::new(MockRuntimeState::default());
    state.activate_circuit_breaker("test failure");
    let control = TelegramControl::new(as_runtime(Arc::clone(&state)));

    let text = control.execute(TelegramCommand::Health);
    assert!(text.contains("DEGRADED"));
    assert!(text.contains("❌")); // Changed from "FAIL" to emoji
}

// =============================================================================
// Additional TelegramControl tests
// =============================================================================

#[test]
fn execute_all_read_only_commands() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(state));

    // All these commands should return non-empty responses
    let commands = [
        TelegramCommand::Start,
        TelegramCommand::Help,
        TelegramCommand::Status,
        TelegramCommand::Health,
        TelegramCommand::Positions,
        TelegramCommand::Stats,
        TelegramCommand::Pool,
        TelegramCommand::Markets,
        TelegramCommand::Version,
    ];

    for cmd in commands {
        let response = control.execute(cmd.clone());
        assert!(
            !response.is_empty(),
            "Command {:?} returned empty response",
            cmd
        );
    }
}

// =============================================================================
// Set Risk Command Tests
// =============================================================================

#[test]
fn set_risk_all_valid_fields() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(Arc::clone(&state)));

    // MinProfitThreshold
    let text = control.execute(TelegramCommand::SetRisk {
        kind: RuntimeRiskLimitKind::MinProfitThreshold,
        value: dec!(0.5),
    });
    assert!(text.contains("Updated min_profit"));
    assert_eq!(state.risk_limits().min_profit_threshold, dec!(0.5));

    // MaxSlippage
    let text = control.execute(TelegramCommand::SetRisk {
        kind: RuntimeRiskLimitKind::MaxSlippage,
        value: dec!(0.1),
    });
    assert!(text.contains("Updated max_slippage"));
    assert_eq!(state.risk_limits().max_slippage, dec!(0.1));

    // MaxPositionPerMarket
    let text = control.execute(TelegramCommand::SetRisk {
        kind: RuntimeRiskLimitKind::MaxPositionPerMarket,
        value: dec!(200),
    });
    assert!(text.contains("Updated max_position"));
    assert_eq!(state.risk_limits().max_position_per_market, dec!(200));

    // MaxTotalExposure
    let text = control.execute(TelegramCommand::SetRisk {
        kind: RuntimeRiskLimitKind::MaxTotalExposure,
        value: dec!(5000),
    });
    assert!(text.contains("Updated max_exposure"));
    assert_eq!(state.risk_limits().max_total_exposure, dec!(5000));
}

#[test]
fn set_risk_validation_errors() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(Arc::clone(&state)));

    // Negative min_profit
    let text = control.execute(TelegramCommand::SetRisk {
        kind: RuntimeRiskLimitKind::MinProfitThreshold,
        value: dec!(-1),
    });
    assert!(text.contains("Error"));

    // Slippage > 1
    let text = control.execute(TelegramCommand::SetRisk {
        kind: RuntimeRiskLimitKind::MaxSlippage,
        value: dec!(1.5),
    });
    assert!(text.contains("Error"));

    // Slippage < 0
    let text = control.execute(TelegramCommand::SetRisk {
        kind: RuntimeRiskLimitKind::MaxSlippage,
        value: dec!(-0.1),
    });
    assert!(text.contains("Error"));

    // Zero max_position
    let text = control.execute(TelegramCommand::SetRisk {
        kind: RuntimeRiskLimitKind::MaxPositionPerMarket,
        value: dec!(0),
    });
    assert!(text.contains("Error"));

    // Negative max_exposure
    let text = control.execute(TelegramCommand::SetRisk {
        kind: RuntimeRiskLimitKind::MaxTotalExposure,
        value: dec!(-100),
    });
    assert!(text.contains("Error"));
}

#[test]
fn set_risk_edge_cases() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(Arc::clone(&state)));

    // Zero min_profit (valid - no minimum)
    let text = control.execute(TelegramCommand::SetRisk {
        kind: RuntimeRiskLimitKind::MinProfitThreshold,
        value: dec!(0),
    });
    assert!(text.contains("Updated"));
    assert_eq!(state.risk_limits().min_profit_threshold, dec!(0));

    // Slippage exactly 0 (valid)
    let text = control.execute(TelegramCommand::SetRisk {
        kind: RuntimeRiskLimitKind::MaxSlippage,
        value: dec!(0),
    });
    assert!(text.contains("Updated"));

    // Slippage exactly 1 (valid - 100%)
    let text = control.execute(TelegramCommand::SetRisk {
        kind: RuntimeRiskLimitKind::MaxSlippage,
        value: dec!(1),
    });
    assert!(text.contains("Updated"));
    assert_eq!(state.risk_limits().max_slippage, dec!(1));

    // Very small but positive position
    let text = control.execute(TelegramCommand::SetRisk {
        kind: RuntimeRiskLimitKind::MaxPositionPerMarket,
        value: dec!(0.001),
    });
    assert!(text.contains("Updated"));

    // Large exposure
    let text = control.execute(TelegramCommand::SetRisk {
        kind: RuntimeRiskLimitKind::MaxTotalExposure,
        value: dec!(1000000),
    });
    assert!(text.contains("Updated"));
    assert_eq!(state.risk_limits().max_total_exposure, dec!(1000000));
}

#[test]
fn set_risk_response_shows_all_limits() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(state));

    let text = control.execute(TelegramCommand::SetRisk {
        kind: RuntimeRiskLimitKind::MinProfitThreshold,
        value: dec!(0.3),
    });

    // Response should show all current limits
    assert!(text.contains("min_profit"));
    assert!(text.contains("max_slippage"));
    assert!(text.contains("max_position"));
    assert!(text.contains("max_exposure"));
}

// =============================================================================
// Status Command Tests
// =============================================================================

#[test]
fn status_shows_active_mode() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(state));

    let text = control.execute(TelegramCommand::Status);
    assert!(text.contains("ACTIVE"));
    assert!(text.contains("▶️"));
}

#[test]
fn status_shows_paused_mode() {
    let state = Arc::new(MockRuntimeState::default());
    state.activate_circuit_breaker("manual pause");
    let control = TelegramControl::new(as_runtime(state));

    let text = control.execute(TelegramCommand::Status);
    assert!(text.contains("PAUSED"));
    assert!(text.contains("⏸️"));
    assert!(text.contains("manual pause"));
}

#[test]
fn status_shows_risk_limits() {
    let state = Arc::new(MockRuntimeState::with_limits(RuntimeRiskLimits {
        max_position_per_market: dec!(50),
        max_total_exposure: dec!(500),
        min_profit_threshold: dec!(0.3),
        max_slippage: dec!(0.02),
    }));
    let control = TelegramControl::new(as_runtime(state));

    let text = control.execute(TelegramCommand::Status);
    assert!(text.contains("Risk Limits"));
    assert!(text.contains("$0.3") || text.contains("0.3")); // min_profit
    assert!(text.contains("2%") || text.contains("2.00%")); // slippage as percentage
    assert!(text.contains("$50")); // max_position
    assert!(text.contains("$500")); // max_exposure
}

#[test]
fn status_shows_pending_info() {
    let state = Arc::new(MockRuntimeState::default());
    state.set_pending(dec!(150), 3);
    let control = TelegramControl::new(as_runtime(state));

    let text = control.execute(TelegramCommand::Status);
    assert!(text.contains("Pending: $150"));
    assert!(text.contains("In-Flight: 3"));
}

// =============================================================================
// Health Command Tests
// =============================================================================

#[test]
fn health_shows_healthy_status() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(state));

    let text = control.execute(TelegramCommand::Health);
    assert!(text.contains("HEALTHY"));
    assert!(text.contains("✅"));
}

#[test]
fn health_shows_exposure_status() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(state));

    let text = control.execute(TelegramCommand::Health);
    assert!(text.contains("Exposure:"));
}

#[test]
fn health_shows_slippage_config() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(state));

    let text = control.execute(TelegramCommand::Health);
    assert!(text.contains("Slippage Config:"));
}

// =============================================================================
// Positions Command Tests
// =============================================================================

#[test]
fn positions_with_multiple_positions() {
    use crate::domain::{
        id::MarketId, id::TokenId, position::Position, position::PositionLeg,
        position::PositionStatus,
    };

    let state = Arc::new(MockRuntimeState::default());

    {
        let mut positions = state.positions_mut();
        for i in 0..3 {
            let legs = vec![
                PositionLeg::new(TokenId::from(format!("yes-{}", i)), dec!(100), dec!(0.40)),
                PositionLeg::new(TokenId::from(format!("no-{}", i)), dec!(100), dec!(0.50)),
            ];
            let position = Position::new(
                positions.next_id(),
                MarketId::from(format!("market-{}", i)),
                legs,
                dec!(90),
                dec!(100),
                chrono::Utc::now(),
                PositionStatus::Open,
            );
            positions.add(position);
        }
    }

    let control = TelegramControl::new(as_runtime(Arc::clone(&state)));
    let text = control.execute(TelegramCommand::Positions);

    assert!(text.contains("Active Positions (3)"));
    assert!(text.contains("market-0"));
    assert!(text.contains("market-1"));
    assert!(text.contains("market-2"));
}

#[test]
fn positions_respects_display_limit() {
    use crate::domain::{
        id::MarketId, id::TokenId, position::Position, position::PositionLeg,
        position::PositionStatus,
    };

    let state = Arc::new(MockRuntimeState::default());
    let pool = database::connection::create_pool("sqlite://:memory:").expect("create pool");
    database::connection::run_migrations(&pool).expect("run migrations");
    let recorder = recorder::create_recorder(pool);
    let runtime = Arc::new(RuntimeStats::new());

    {
        let mut positions = state.positions_mut();
        for i in 0..15 {
            let legs = vec![PositionLeg::new(
                TokenId::from(format!("yes-{}", i)),
                dec!(100),
                dec!(0.40),
            )];
            let position = Position::new(
                positions.next_id(),
                MarketId::from(format!("market-{}", i)),
                legs,
                dec!(40),
                dec!(100),
                chrono::Utc::now(),
                PositionStatus::Open,
            );
            positions.add(position);
        }
    }

    // Use limit of 5
    let control =
        TelegramControl::with_config(as_runtime(Arc::clone(&state)), recorder, runtime, 5);
    let text = control.execute(TelegramCommand::Positions);

    assert!(text.contains("Active Positions (15)"));
    assert!(text.contains("... and 10 more"));
}

#[test]
fn positions_shows_different_statuses() {
    use crate::domain::{
        id::MarketId, id::TokenId, position::Position, position::PositionLeg,
        position::PositionStatus,
    };

    let state = Arc::new(MockRuntimeState::default());

    {
        let mut positions = state.positions_mut();

        // Open position
        let open_legs = vec![PositionLeg::new(
            TokenId::from("yes-open"),
            dec!(100),
            dec!(0.40),
        )];
        let open_id = positions.next_id();
        positions.add(Position::new(
            open_id,
            MarketId::from("market-open"),
            open_legs,
            dec!(40),
            dec!(100),
            chrono::Utc::now(),
            PositionStatus::Open,
        ));

        // Partial fill position
        let partial_legs = vec![PositionLeg::new(
            TokenId::from("yes-partial"),
            dec!(50),
            dec!(0.40),
        )];
        let partial_id = positions.next_id();
        positions.add(Position::new(
            partial_id,
            MarketId::from("market-partial"),
            partial_legs,
            dec!(20),
            dec!(50),
            chrono::Utc::now(),
            PositionStatus::PartialFill {
                filled: vec![],
                missing: vec![],
            },
        ));
    }

    let control = TelegramControl::new(as_runtime(state));
    let text = control.execute(TelegramCommand::Positions);

    assert!(text.contains("open"));
    assert!(text.contains("partial"));
    assert!(text.contains("🟢")); // Open emoji
    assert!(text.contains("🟡")); // Partial emoji
}

// =============================================================================
// RuntimeStats Tests
// =============================================================================

#[test]
fn runtime_stats_default_values() {
    let stats = RuntimeStats::new();
    assert!(stats.pool_stats().is_none());
    assert_eq!(stats.market_count(), 0);
    assert_eq!(stats.token_count(), 0);
    assert!(stats.cluster_view().is_none());
}

#[test]
fn runtime_stats_atomic_updates() {
    let stats = RuntimeStats::new();

    stats.update_market_counts(100, 200);
    assert_eq!(stats.market_count(), 100);
    assert_eq!(stats.token_count(), 200);

    stats.update_market_counts(50, 100);
    assert_eq!(stats.market_count(), 50);
    assert_eq!(stats.token_count(), 100);
}

#[test]
fn runtime_stats_pool_stats_update() {
    let stats = RuntimeStats::new();

    let pool_stats = PoolStats {
        active_connections: 10,
        total_rotations: 100,
        total_restarts: 5,
        events_dropped: 50,
    };

    stats.update_pool_stats(pool_stats);

    let retrieved = stats.pool_stats().unwrap();
    assert_eq!(retrieved.active_connections, 10);
    assert_eq!(retrieved.total_rotations, 100);
    assert_eq!(retrieved.total_restarts, 5);
    assert_eq!(retrieved.events_dropped, 50);
}

// =============================================================================
// Format Uptime Tests
// =============================================================================

#[test]
fn format_uptime_zero() {
    use chrono::Utc;
    let now = Utc::now();
    let uptime = format_uptime(now);
    assert!(uptime.starts_with("00:00:0")); // Allow for slight timing variance
}

#[test]
fn format_uptime_hours() {
    use chrono::{Duration, Utc};
    let started = Utc::now() - Duration::hours(5);
    let uptime = format_uptime(started);
    assert!(uptime.starts_with("05:"));
}

#[test]
fn format_uptime_days() {
    use chrono::{Duration, Utc};
    let started = Utc::now() - Duration::days(2) - Duration::hours(3);
    let uptime = format_uptime(started);
    // 2 days = 48 hours + 3 = 51 hours
    assert!(uptime.starts_with("51:"));
}

#[test]
fn format_uptime_format_is_correct() {
    use chrono::{Duration, Utc};
    let started = Utc::now() - Duration::hours(1) - Duration::minutes(23) - Duration::seconds(45);
    let uptime = format_uptime(started);

    // Should be in HH:MM:SS format
    let parts: Vec<&str> = uptime.split(':').collect();
    assert_eq!(parts.len(), 3);

    // Check each part is numeric
    for part in parts {
        assert!(part.parse::<u32>().is_ok());
    }
}

// =============================================================================
// TelegramControl Constructor Tests
// =============================================================================

#[test]
fn telegram_control_new_creates_minimal_control() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(state));

    // Stats and pool should not be available
    let stats_text = control.execute(TelegramCommand::Stats);
    assert!(stats_text.contains("not available"));

    let pool_text = control.execute(TelegramCommand::Pool);
    assert!(pool_text.contains("not available"));
}

#[test]
fn telegram_control_with_config_has_full_features() {
    let state = Arc::new(MockRuntimeState::default());
    let pool = database::connection::create_pool("sqlite://:memory:").expect("create pool");
    database::connection::run_migrations(&pool).expect("run migrations");
    let recorder = recorder::create_recorder(pool);
    let runtime = Arc::new(RuntimeStats::new());

    let control = TelegramControl::with_config(as_runtime(state), recorder, runtime, 10);

    // Stats should be available
    let stats_text = control.execute(TelegramCommand::Stats);
    assert!(stats_text.contains("Today's Statistics"));
}

// =============================================================================
// Clone Tests
// =============================================================================

#[test]
fn telegram_control_clone_shares_state() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(Arc::clone(&state)));
    let cloned = control.clone();

    // Pause on original
    let _ = control.execute(TelegramCommand::Pause);

    // Should be visible on clone
    let status = cloned.execute(TelegramCommand::Status);
    assert!(status.contains("PAUSED"));
}

// =============================================================================
// Version Command Tests
// =============================================================================

#[test]
fn version_contains_version_number() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(state));

    let text = control.execute(TelegramCommand::Version);
    assert!(text.contains("Version"));
    // Version should contain the cargo package version
    let version = env!("CARGO_PKG_VERSION");
    assert!(text.contains(version));
}

// =============================================================================
// Start and Help Commands
// =============================================================================

#[test]
fn start_and_help_are_identical() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(state));

    let start_text = control.execute(TelegramCommand::Start);
    let help_text = control.execute(TelegramCommand::Help);

    assert_eq!(start_text, help_text);
}

#[test]
fn help_contains_all_commands() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(state));

    let text = control.execute(TelegramCommand::Help);

    // Should list all available commands
    assert!(text.contains("/status"));
    assert!(text.contains("/health"));
    assert!(text.contains("/positions"));
    assert!(text.contains("/stats"));
    assert!(text.contains("/pool"));
    assert!(text.contains("/markets"));
    assert!(text.contains("/version"));
    assert!(text.contains("/pause"));
    assert!(text.contains("/resume"));
    assert!(text.contains("/set_risk"));
}

// =============================================================================
// Pause/Resume Idempotency
// =============================================================================

#[test]
fn multiple_pauses_are_safe() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(Arc::clone(&state)));

    // First pause
    let text1 = control.execute(TelegramCommand::Pause);
    assert!(text1.contains("paused"));
    assert!(state.is_circuit_breaker_active());

    // Second pause - should indicate already paused
    let text2 = control.execute(TelegramCommand::Pause);
    assert!(text2.contains("Already paused"));
    assert!(state.is_circuit_breaker_active());

    // Third pause
    let text3 = control.execute(TelegramCommand::Pause);
    assert!(text3.contains("Already paused"));
}

#[test]
fn multiple_resumes_are_safe() {
    let state = Arc::new(MockRuntimeState::default());
    let control = TelegramControl::new(as_runtime(Arc::clone(&state)));

    // First resume (already active)
    let text1 = control.execute(TelegramCommand::Resume);
    assert!(text1.contains("already active"));
    assert!(!state.is_circuit_breaker_active());

    // Second resume
    let text2 = control.execute(TelegramCommand::Resume);
    assert!(text2.contains("already active"));
    assert!(!state.is_circuit_breaker_active());
}
