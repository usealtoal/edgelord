//! Execution spawning and position recording.

use std::sync::Arc;

use tracing::{error, info, warn};

use crate::app::state::AppState;
use crate::app::status::StatusWriter;
use crate::core::domain::{Opportunity, Position, PositionLeg, PositionStatus, TokenId};
use crate::core::exchange::{
    ArbitrageExecutionResult, ArbitrageExecutor, FailedLeg, FilledLeg, OrderId,
};
use crate::core::service::{Event, ExecutionEvent, NotifierRegistry};
use rust_decimal::Decimal;

/// Spawn async execution without blocking message processing.
pub(crate) fn spawn_execution(
    executor: Arc<dyn ArbitrageExecutor + Send + Sync>,
    opportunity: Opportunity,
    notifiers: Arc<NotifierRegistry>,
    state: Arc<AppState>,
    status_writer: Option<Arc<StatusWriter>>,
) {
    let market_id = opportunity.market_id().to_string();
    let expected_profit = opportunity.expected_profit();

    tokio::spawn(async move {
        let result = executor.execute_arbitrage(&opportunity).await;

        // Always release the execution lock
        state.release_execution(&market_id);

        match result {
            Ok(exec_result) => {
                match &exec_result {
                    ArbitrageExecutionResult::Success { .. } => {
                        record_position(&state, &opportunity);
                        // Record execution with profit in status file
                        if let Some(ref writer) = status_writer {
                            writer.record_execution(expected_profit);
                            // Update runtime stats
                            let positions = state.positions();
                            let open_count = positions.open_positions().count();
                            let exposure = positions.total_exposure();
                            let max_exposure = state.risk_limits().max_total_exposure;
                            writer.update_runtime(open_count, exposure, max_exposure);
                            if let Err(e) = writer.write() {
                                warn!(error = %e, "Failed to write status file");
                            }
                        }
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
                        for fill in filled {
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
                            record_partial_position(&state, &opportunity, filled, failed);
                            if let Some(ref writer) = status_writer {
                                let positions = state.positions();
                                let open_count = positions.open_positions().count();
                                let exposure = positions.total_exposure();
                                let max_exposure = state.risk_limits().max_total_exposure;
                                writer.update_runtime(open_count, exposure, max_exposure);
                                if let Err(e) = writer.write() {
                                    warn!(error = %e, "Failed to write status file");
                                }
                            }
                        } else {
                            info!("Successfully cancelled all filled legs, no position recorded");
                        }
                    }
                    ArbitrageExecutionResult::Failed { .. } => {}
                }

                // Notify execution result
                notifiers.notify_all(Event::ExecutionCompleted(ExecutionEvent::from_result(
                    &market_id,
                    &exec_result,
                )));
            }
            Err(e) => {
                error!(error = %e, "Execution failed");
                notifiers.notify_all(Event::ExecutionCompleted(ExecutionEvent {
                    market_id,
                    success: false,
                    details: e.to_string(),
                }));
            }
        }
    });
}

/// Record a position in shared state.
pub(crate) fn record_position(state: &AppState, opportunity: &Opportunity) {
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
    let position = Position::new(
        positions.next_id(),
        opportunity.market_id().clone(),
        position_legs,
        opportunity.total_cost() * opportunity.volume(),
        opportunity.volume(),
        chrono::Utc::now(),
        PositionStatus::Open,
    );
    positions.add(position);
}

/// Record a partial fill position.
pub(crate) fn record_partial_position(
    state: &AppState,
    opportunity: &Opportunity,
    filled: &[FilledLeg],
    failed: &[FailedLeg],
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
    let position = Position::new(
        positions.next_id(),
        opportunity.market_id().clone(),
        position_legs,
        entry_cost,
        opportunity.volume(),
        chrono::Utc::now(),
        PositionStatus::PartialFill {
            filled: filled_token_ids,
            missing: missing_token_ids,
        },
    );
    positions.add(position);
}
