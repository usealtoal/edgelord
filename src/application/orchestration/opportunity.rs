//! Opportunity decision flow for orchestration.

use std::sync::Arc;

use tracing::{debug, info};

use super::execution::spawn_execution;
use super::handler::OpportunityHandlingContext;
use super::slippage::get_max_slippage;
use crate::domain::{opportunity::Opportunity, stats::RecordedOpportunity};
use crate::error::RiskError;
use crate::port::inbound::risk::RiskCheckResult;
use crate::port::outbound::notifier::{Event, OpportunityEvent, RiskEvent};

/// Handle a detected opportunity.
pub(crate) fn handle_opportunity(opp: Opportunity, context: OpportunityHandlingContext<'_>) {
    let OpportunityHandlingContext {
        executor,
        risk_manager,
        notifiers,
        state,
        stats,
        cache,
        dry_run,
    } = context;

    if !state.try_lock_execution(opp.market_id().as_str()) {
        debug!(market_id = %opp.market_id(), "Execution already in progress, skipping");
        return;
    }

    let max_slippage = state.risk_limits().max_slippage;
    if let Some(slippage) = get_max_slippage(&opp, cache) {
        if slippage > max_slippage {
            debug!(
                market_id = %opp.market_id(),
                slippage = %slippage,
                max = %max_slippage,
                "Slippage check failed, rejecting opportunity"
            );
            state.release_execution(opp.market_id().as_str());

            stats.record_opportunity(&RecordedOpportunity {
                strategy: opp.strategy().to_string(),
                market_ids: vec![opp.market_id().to_string()],
                edge: opp.edge(),
                expected_profit: opp.expected_profit(),
                executed: false,
                rejected_reason: Some("slippage_too_high".to_string()),
            });

            let error = RiskError::SlippageTooHigh {
                actual: slippage,
                max: max_slippage,
            };
            notifiers.notify_all(Event::RiskRejected(RiskEvent::new(
                opp.market_id().as_str(),
                &error,
            )));
            return;
        }
    }

    notifiers.notify_all(Event::OpportunityDetected(OpportunityEvent::from(&opp)));

    match risk_manager.check(&opp) {
        RiskCheckResult::Approved => {
            let opp_id = stats.record_opportunity(&RecordedOpportunity {
                strategy: opp.strategy().to_string(),
                market_ids: vec![opp.market_id().to_string()],
                edge: opp.edge(),
                expected_profit: opp.expected_profit(),
                executed: !dry_run,
                rejected_reason: None,
            });

            let reserved_exposure = opp.total_cost() * opp.volume();

            if dry_run {
                info!(
                    market_id = %opp.market_id(),
                    edge = %opp.edge(),
                    profit = %opp.expected_profit(),
                    "Dry-run: would execute trade"
                );
                state.release_exposure(reserved_exposure);
                state.release_execution(opp.market_id().as_str());
            } else if let Some(exec) = executor {
                spawn_execution(
                    exec,
                    opp,
                    notifiers.clone(),
                    state.clone(),
                    Arc::clone(stats),
                    opp_id,
                );
            } else {
                state.release_exposure(reserved_exposure);
                state.release_execution(opp.market_id().as_str());
            }
        }
        RiskCheckResult::Rejected(error) => {
            stats.record_opportunity(&RecordedOpportunity {
                strategy: opp.strategy().to_string(),
                market_ids: vec![opp.market_id().to_string()],
                edge: opp.edge(),
                expected_profit: opp.expected_profit(),
                executed: false,
                rejected_reason: Some(format!("{error}")),
            });

            state.release_execution(opp.market_id().as_str());
            notifiers.notify_all(Event::RiskRejected(RiskEvent::new(
                opp.market_id().as_str(),
                &error,
            )));
        }
    }
}
