//! Position recording helpers for execution flow.

use rust_decimal::Decimal;

use crate::application::state::AppState;
use crate::domain::id::TokenId;
use crate::domain::opportunity::Opportunity;
use crate::domain::position::{Position, PositionLeg, PositionStatus};
use crate::domain::trade::{Failure, Fill};

/// Record a fully executed position in shared state.
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

/// Record a partial-fill position in shared state.
pub(crate) fn record_partial_position(
    state: &AppState,
    opportunity: &Opportunity,
    fills: &[Fill],
    failures: &[Failure],
    trade_id: Option<i32>,
) {
    let filled_token_ids: Vec<TokenId> = fills.iter().map(|f| f.token_id.clone()).collect();
    let missing_token_ids: Vec<TokenId> = failures.iter().map(|f| f.token_id.clone()).collect();

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
