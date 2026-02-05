//! Execution spawning and position recording.

use std::sync::Arc;

use rust_decimal::Decimal;
use tracing::{error, info, warn};

use crate::app::state::AppState;
use crate::core::domain::{
    ArbitrageExecutionResult, FailedLeg, FilledLeg, Opportunity, OrderId, Position, PositionLeg,
    PositionStatus, TokenId,
};
use crate::core::exchange::ArbitrageExecutor;
use crate::core::service::stats::{StatsRecorder, TradeLeg, TradeOpenEvent};
use crate::core::service::{Event, ExecutionEvent, NotifierRegistry};

/// Spawn async execution without blocking message processing.
pub fn spawn_execution(
    executor: Arc<dyn ArbitrageExecutor + Send + Sync>,
    opportunity: Opportunity,
    notifiers: Arc<NotifierRegistry>,
    state: Arc<AppState>,
    stats: Arc<StatsRecorder>,
    opportunity_id: Option<i32>,
) {
    let market_id = opportunity.market_id().to_string();

    tokio::spawn(async move {
        let result = executor.execute_arbitrage(&opportunity).await;

        // Always release the execution lock
        state.release_execution(&market_id);

        match result {
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
        opportunity.volume(),
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
        opportunity.volume(),
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
