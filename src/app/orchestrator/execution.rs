//! Execution spawning and position recording.

use std::sync::Arc;

use rust_decimal::Decimal;
use tokio::time::{timeout, Duration};
use tracing::{error, info, warn};

use crate::app::state::AppState;
use crate::core::domain::{
    ArbitrageExecutionResult, FailedLeg, FilledLeg, Opportunity, OrderId, Position, PositionLeg,
    PositionStatus, TokenId,
};
use crate::core::exchange::ArbitrageExecutor;
use crate::core::service::statistics::{StatsRecorder, TradeLeg, TradeOpenEvent};
use crate::core::service::{Event, ExecutionEvent, NotifierRegistry};

struct ExecutionLockGuard {
    state: Arc<AppState>,
    market_id: String,
}

impl ExecutionLockGuard {
    fn new(state: Arc<AppState>, market_id: String) -> Self {
        Self { state, market_id }
    }
}

impl Drop for ExecutionLockGuard {
    fn drop(&mut self) {
        self.state.release_execution(&self.market_id);
    }
}

/// Spawn async execution without blocking message processing.
pub(crate) fn spawn_execution(
    executor: Arc<dyn ArbitrageExecutor + Send + Sync>,
    opportunity: Opportunity,
    notifiers: Arc<NotifierRegistry>,
    state: Arc<AppState>,
    stats: Arc<StatsRecorder>,
    opportunity_id: Option<i32>,
) {
    let market_id = opportunity.market_id().to_string();

    tokio::spawn(async move {
        let _lock_guard = ExecutionLockGuard::new(Arc::clone(&state), market_id.clone());
        // Calculate reserved exposure for release
        let reserved_exposure = opportunity.total_cost() * opportunity.volume();

        // Get configurable timeout from state (with test override)
        #[cfg(test)]
        let execution_timeout = Duration::from_millis(100);
        #[cfg(not(test))]
        let execution_timeout = Duration::from_secs(state.risk_limits().execution_timeout_secs);

        let result = timeout(execution_timeout, executor.execute_arbitrage(&opportunity)).await;

        match result {
            Ok(exec_result) => match exec_result {
                Ok(exec_result) => {
                    match &exec_result {
                        ArbitrageExecutionResult::Success { filled: _ } => {
                            // Record trade open first to get trade_id
                            let trade_id = if let Some(opp_id) = opportunity_id {
                                let legs: Vec<TradeLeg> = opportunity
                                    .legs()
                                    .iter()
                                    .map(|leg| TradeLeg {
                                        token_id: leg.token_id().to_string(),
                                        side: "buy".to_string(),
                                        price: leg.ask_price(),
                                        size: opportunity.volume(),
                                    })
                                    .collect();

                                stats.record_trade_open(&TradeOpenEvent {
                                    opportunity_id: opp_id,
                                    strategy: opportunity.strategy().to_string(),
                                    market_ids: vec![market_id.clone()],
                                    legs,
                                    size: opportunity.volume(),
                                    expected_profit: opportunity.expected_profit(),
                                })
                            } else {
                                None
                            };

                            // Record position with trade_id for close tracking
                            record_position(&state, &opportunity, trade_id);

                            // Release reserved exposure (now converted to actual position exposure)
                            state.release_exposure(reserved_exposure);

                            // Track peak exposure
                            let exposure = state.total_exposure();
                            stats.update_peak_exposure(exposure);
                        }
                        ArbitrageExecutionResult::PartialFill { filled, failed } => {
                            let filled_ids: Vec<_> =
                                filled.iter().map(|f| f.token_id.to_string()).collect();
                            let failed_ids: Vec<_> =
                                failed.iter().map(|f| f.token_id.to_string()).collect();
                            warn!(
                                filled = ?filled_ids,
                                failed = ?failed_ids,
                                "Partial fill detected, attempting recovery"
                            );

                            // Try to cancel all filled orders
                            let mut cancel_failed = false;
                            for fill in filled.iter() {
                                let order_id = OrderId::new(fill.order_id.clone());
                                if let Err(cancel_err) =
                                    ArbitrageExecutor::cancel(executor.as_ref(), &order_id).await
                                {
                                    warn!(error = %cancel_err, token = %fill.token_id, "Failed to cancel filled leg");
                                    cancel_failed = true;
                                }
                            }

                            if cancel_failed {
                                warn!("Some cancellations failed, recording partial position");
                                record_partial_position(&state, &opportunity, filled, failed, None);
                                // Release reserved exposure (partial position recorded)
                                state.release_exposure(reserved_exposure);
                            } else {
                                info!(
                                    "Successfully cancelled all filled legs, no position recorded"
                                );
                                // Release reserved exposure (no position recorded)
                                state.release_exposure(reserved_exposure);
                            }
                        }
                        ArbitrageExecutionResult::Failed { .. } => {
                            // Release reserved exposure on failure
                            state.release_exposure(reserved_exposure);
                        }
                    }

                    // Notify execution result
                    notifiers.notify_all(Event::ExecutionCompleted(ExecutionEvent::from_result(
                        &market_id,
                        &exec_result,
                    )));
                }
                Err(e) => {
                    error!(error = %e, "Execution failed");
                    // Release reserved exposure on error
                    state.release_exposure(reserved_exposure);
                    notifiers.notify_all(Event::ExecutionCompleted(ExecutionEvent {
                        market_id,
                        success: false,
                        details: e.to_string(),
                    }));
                }
            },
            Err(_) => {
                error!(market_id = %market_id, "Execution timed out");
                // Release reserved exposure on timeout
                state.release_exposure(reserved_exposure);
                notifiers.notify_all(Event::ExecutionCompleted(ExecutionEvent {
                    market_id,
                    success: false,
                    details: "execution_timeout".to_string(),
                }));
            }
        }
    });
}

/// Record a position in shared state.
pub(crate) fn record_position(state: &AppState, opportunity: &Opportunity, trade_id: Option<i32>) {
    let position_legs: Vec<PositionLeg> = opportunity
        .legs()
        .iter()
        .map(|leg| {
            PositionLeg::new(
                leg.token_id().clone(),
                opportunity.volume(),
                leg.ask_price(),
            )
        })
        .collect();

    let mut positions = state.positions_mut();
    let mut position = Position::new(
        positions.next_id(),
        opportunity.market_id().clone(),
        position_legs,
        opportunity.total_cost() * opportunity.volume(),
        opportunity.payout() * opportunity.volume(),
        chrono::Utc::now(),
        PositionStatus::Open,
    );

    if let Some(tid) = trade_id {
        position = position.with_trade_id(tid);
    }

    positions.add(position);
}

/// Record a partial fill position.
pub(crate) fn record_partial_position(
    state: &AppState,
    opportunity: &Opportunity,
    filled: &[FilledLeg],
    failed: &[FailedLeg],
    trade_id: Option<i32>,
) {
    let filled_token_ids: Vec<TokenId> = filled.iter().map(|f| f.token_id.clone()).collect();
    let missing_token_ids: Vec<TokenId> = failed.iter().map(|f| f.token_id.clone()).collect();

    // Build position legs from filled legs
    let position_legs: Vec<PositionLeg> = opportunity
        .legs()
        .iter()
        .filter(|leg| filled_token_ids.contains(leg.token_id()))
        .map(|leg| {
            PositionLeg::new(
                leg.token_id().clone(),
                opportunity.volume(),
                leg.ask_price(),
            )
        })
        .collect();

    let entry_cost: Decimal = position_legs
        .iter()
        .map(|l| l.entry_price() * l.size())
        .sum();

    let mut positions = state.positions_mut();
    let mut position = Position::new(
        positions.next_id(),
        opportunity.market_id().clone(),
        position_legs,
        entry_cost,
        opportunity.payout() * opportunity.volume(),
        chrono::Utc::now(),
        PositionStatus::PartialFill {
            filled: filled_token_ids,
            missing: missing_token_ids,
        },
    );

    if let Some(tid) = trade_id {
        position = position.with_trade_id(tid);
    }

    positions.add(position);
}

#[cfg(test)]
mod tests {
    use super::spawn_execution;
    use std::future::pending;
    use std::sync::Arc;

    use async_trait::async_trait;
    use rust_decimal_macros::dec;
    use tokio::time::{sleep, Duration, Instant};

    use crate::app::AppState;
    use crate::core::domain::{
        ArbitrageExecutionResult, FailedLeg, FilledLeg, MarketId, Opportunity, OpportunityLeg,
        OrderId, PositionStatus, TokenId,
    };
    use crate::core::exchange::ArbitrageExecutor;
    use crate::core::service::{statistics, NotifierRegistry};
    use crate::error::{Error, ExecutionError};

    /// Mock executor that returns PartialFill and fails cancel on one leg.
    struct MockPartialFillExecutor {
        /// Order IDs that should fail cancellation.
        cancel_fail_order_ids: Vec<String>,
    }

    #[async_trait]
    impl ArbitrageExecutor for MockPartialFillExecutor {
        async fn execute_arbitrage(
            &self,
            _opportunity: &Opportunity,
        ) -> Result<ArbitrageExecutionResult, Error> {
            Ok(ArbitrageExecutionResult::PartialFill {
                filled: vec![
                    FilledLeg::new(TokenId::from("token-1"), "order-1"),
                    FilledLeg::new(TokenId::from("token-2"), "order-2"),
                ],
                failed: vec![FailedLeg::new(TokenId::from("token-3"), "execution failed")],
            })
        }

        async fn cancel(&self, order_id: &OrderId) -> Result<(), Error> {
            if self
                .cancel_fail_order_ids
                .contains(&order_id.as_str().to_string())
            {
                Err(Error::Execution(ExecutionError::OrderRejected(
                    "cancel failed".to_string(),
                )))
            } else {
                Ok(())
            }
        }

        fn exchange_name(&self) -> &'static str {
            "mock"
        }
    }

    /// Mock executor that never returns to simulate a hang.
    struct MockHangingExecutor;

    #[async_trait]
    impl ArbitrageExecutor for MockHangingExecutor {
        async fn execute_arbitrage(
            &self,
            _opportunity: &Opportunity,
        ) -> Result<ArbitrageExecutionResult, Error> {
            pending::<()>().await;
            unreachable!("pending should never resolve");
        }

        async fn cancel(&self, _order_id: &OrderId) -> Result<(), Error> {
            Ok(())
        }

        fn exchange_name(&self) -> &'static str {
            "mock"
        }
    }

    #[tokio::test]
    async fn partial_fill_triggers_cancel_and_records_partial_position_on_failure() {
        let executor = Arc::new(MockPartialFillExecutor {
            cancel_fail_order_ids: vec!["order-1".to_string()],
        });

        let opportunity = Opportunity::with_strategy(
            MarketId::from("test-market"),
            "Test question?",
            vec![
                OpportunityLeg::new(TokenId::from("token-1"), dec!(0.40)),
                OpportunityLeg::new(TokenId::from("token-2"), dec!(0.50)),
                OpportunityLeg::new(TokenId::from("token-3"), dec!(0.10)),
            ],
            dec!(100),
            dec!(1.00),
            "test-strategy",
        );

        let state = Arc::new(AppState::default());
        let notifiers = Arc::new(NotifierRegistry::new());
        let db_pool = crate::core::db::create_pool("sqlite://:memory:").unwrap();
        let stats = statistics::create_recorder(db_pool);

        spawn_execution(
            executor,
            opportunity.clone(),
            notifiers,
            state.clone(),
            stats,
            None,
        );

        let start = Instant::now();
        let timeout = Duration::from_secs(5);
        loop {
            if state.positions().all().next().is_some() {
                break;
            }
            if start.elapsed() > timeout {
                panic!("Timed out waiting for partial fill position");
            }
            sleep(Duration::from_millis(10)).await;
        }

        let positions = state.positions();
        let all_positions: Vec<_> = positions.all().collect();
        assert_eq!(
            all_positions.len(),
            1,
            "Expected exactly one position to be recorded"
        );

        let position = &all_positions[0];
        assert_eq!(
            position.market_id().as_str(),
            "test-market",
            "Position should have correct market ID"
        );

        match position.status() {
            PositionStatus::PartialFill { filled, missing } => {
                assert_eq!(filled.len(), 2, "Should have 2 filled legs");
                assert_eq!(missing.len(), 1, "Should have 1 missing leg");

                let filled_ids: Vec<&str> = filled.iter().map(|t| t.as_str()).collect();
                assert!(
                    filled_ids.contains(&"token-1"),
                    "token-1 should be in filled"
                );
                assert!(
                    filled_ids.contains(&"token-2"),
                    "token-2 should be in filled"
                );

                let missing_ids: Vec<&str> = missing.iter().map(|t| t.as_str()).collect();
                assert!(
                    missing_ids.contains(&"token-3"),
                    "token-3 should be in missing"
                );
            }
            _ => panic!(
                "Position should have PartialFill status, got {:?}",
                position.status()
            ),
        }
    }

    #[tokio::test]
    async fn execution_timeout_releases_lock() {
        let executor = Arc::new(MockHangingExecutor);
        let opportunity = Opportunity::with_strategy(
            MarketId::from("timeout-market"),
            "Timeout test?",
            vec![
                OpportunityLeg::new(TokenId::from("token-1"), dec!(0.40)),
                OpportunityLeg::new(TokenId::from("token-2"), dec!(0.50)),
            ],
            dec!(100),
            dec!(1.00),
            "test-strategy",
        );

        let state = Arc::new(AppState::default());
        let notifiers = Arc::new(NotifierRegistry::new());
        let db_pool = crate::core::db::create_pool("sqlite://:memory:").unwrap();
        let stats = statistics::create_recorder(db_pool);

        assert!(state.try_lock_execution("timeout-market"));

        spawn_execution(executor, opportunity, notifiers, state.clone(), stats, None);

        let start = Instant::now();
        let timeout = Duration::from_secs(1);
        loop {
            if state.try_lock_execution("timeout-market") {
                state.release_execution("timeout-market");
                break;
            }
            if start.elapsed() > timeout {
                panic!("Execution lock was not released after timeout");
            }
            sleep(Duration::from_millis(10)).await;
        }
    }
}
