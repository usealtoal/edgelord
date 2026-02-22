use super::super::command::TelegramCommand;
use super::*;
use parking_lot::RwLock;
use rust_decimal_macros::dec;

use crate::adapter::outbound::sqlite::database;
use crate::adapter::outbound::sqlite::stats_recorder;
use crate::port::inbound::runtime::{
    RuntimePosition, RuntimePositionStatus, RuntimeRiskLimitKind, RuntimeRiskLimitUpdateError,
    RuntimeRiskLimits, RuntimeState,
};

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
        }
    }
}

impl MockRuntimeState {
    fn positions_mut(&self) -> parking_lot::RwLockWriteGuard<'_, TestPositionStore> {
        self.positions.write()
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
        rust_decimal::Decimal::ZERO
    }

    fn pending_execution_count(&self) -> usize {
        0
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
    let recorder = stats_recorder::create_recorder(pool);

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
    let recorder = stats_recorder::create_recorder(pool);
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
    let recorder = stats_recorder::create_recorder(pool);
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
    let recorder = stats_recorder::create_recorder(pool);
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
    let recorder = stats_recorder::create_recorder(pool);
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
