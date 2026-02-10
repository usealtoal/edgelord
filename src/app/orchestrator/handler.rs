//! Event and opportunity handling.

use std::sync::Arc;
use std::time::Instant;

use rust_decimal::Decimal;
use tracing::{debug, info, warn};

use super::execution::spawn_execution;
use crate::app::state::AppState;
use crate::core::cache::OrderBookCache;
use crate::core::domain::{MarketRegistry, Opportunity};
use crate::core::exchange::{ArbitrageExecutor, MarketEvent};
use crate::core::service::position::{CloseReason, PositionManager};
use crate::core::service::statistics::{RecordedOpportunity, StatsRecorder};
use crate::core::service::{
    Event, NotifierRegistry, OpportunityEvent, RiskCheckResult, RiskEvent, RiskManager,
};
use crate::core::strategy::{DetectionContext, StrategyRegistry};
use crate::error::RiskError;

/// Handle incoming market events from the data stream.
#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_market_event(
    event: MarketEvent,
    cache: &OrderBookCache,
    registry: &MarketRegistry,
    strategies: &StrategyRegistry,
    executor: Option<Arc<dyn ArbitrageExecutor + Send + Sync>>,
    risk_manager: &RiskManager,
    notifiers: &Arc<NotifierRegistry>,
    state: &Arc<AppState>,
    stats: &Arc<StatsRecorder>,
    position_manager: &Arc<PositionManager>,
    dry_run: bool,
) {
    let start = Instant::now();

    match event {
        MarketEvent::OrderBookSnapshot { token_id, book } => {
            cache.update(book);

            if let Some(market) = registry.get_by_token(&token_id) {
                let ctx = DetectionContext::new(market, cache);
                let opportunities = strategies.detect_all(&ctx);

                debug!(
                    market_id = %market.market_id(),
                    opportunities_found = opportunities.len(),
                    "Strategy detection complete (snapshot)"
                );

                for opp in opportunities {
                    handle_opportunity(
                        opp,
                        executor.clone(),
                        risk_manager,
                        notifiers,
                        state,
                        stats,
                        cache,
                        dry_run,
                    );
                }
            }

            // Record processing latency
            let elapsed = start.elapsed();
            stats.record_latency(elapsed.as_millis() as u32);
        }
        MarketEvent::OrderBookDelta { token_id, book } => {
            // For now, treat deltas as snapshots (simple approach)
            cache.update(book);

            if let Some(market) = registry.get_by_token(&token_id) {
                let ctx = DetectionContext::new(market, cache);
                let opportunities = strategies.detect_all(&ctx);

                debug!(
                    market_id = %market.market_id(),
                    opportunities_found = opportunities.len(),
                    "Strategy detection complete (delta)"
                );

                for opp in opportunities {
                    handle_opportunity(
                        opp,
                        executor.clone(),
                        risk_manager,
                        notifiers,
                        state,
                        stats,
                        cache,
                        dry_run,
                    );
                }
            }

            // Record processing latency
            let elapsed = start.elapsed();
            stats.record_latency(elapsed.as_millis() as u32);
        }
        MarketEvent::MarketSettled {
            market_id,
            winning_outcome,
            payout_per_share,
        } => {
            info!(
                market_id = %market_id,
                winning_outcome = %winning_outcome,
                payout = %payout_per_share,
                "Market settled"
            );

            // Close all positions for this market
            let mut tracker = state.positions_mut();
            let total_pnl = position_manager.close_all_for_market(
                &mut tracker,
                &market_id,
                |pos| PositionManager::calculate_arbitrage_pnl(pos, payout_per_share),
                CloseReason::Settlement {
                    winning_outcome: winning_outcome.clone(),
                },
            );

            if total_pnl != Decimal::ZERO {
                info!(
                    market_id = %market_id,
                    total_pnl = %total_pnl,
                    "Positions settled"
                );
            }
        }
        MarketEvent::Connected => {
            info!("Data stream connected");
        }
        MarketEvent::Disconnected { reason } => {
            warn!(reason = %reason, "Data stream disconnected");
        }
    }
}

/// Handle a detected opportunity.
pub(crate) fn handle_opportunity(
    opp: Opportunity,
    executor: Option<Arc<dyn ArbitrageExecutor + Send + Sync>>,
    risk_manager: &RiskManager,
    notifiers: &Arc<NotifierRegistry>,
    state: &Arc<AppState>,
    stats: &Arc<StatsRecorder>,
    cache: &OrderBookCache,
    dry_run: bool,
) {
    // Check for duplicate execution
    if !state.try_lock_execution(opp.market_id().as_str()) {
        debug!(market_id = %opp.market_id(), "Execution already in progress, skipping");
        return;
    }

    // Pre-execution slippage check
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

            // Record rejected opportunity
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

    // Notify opportunity detected
    notifiers.notify_all(Event::OpportunityDetected(OpportunityEvent::from(&opp)));

    // Check risk
    match risk_manager.check(&opp) {
        RiskCheckResult::Approved => {
            // Record approved opportunity
            let opp_id = stats.record_opportunity(&RecordedOpportunity {
                strategy: opp.strategy().to_string(),
                market_ids: vec![opp.market_id().to_string()],
                edge: opp.edge(),
                expected_profit: opp.expected_profit(),
                executed: !dry_run,
                rejected_reason: None,
            });

            // Exposure was reserved by risk check, calculate amount for release
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
                // Exposure will be released when execution completes
            } else {
                // No executor, release the reserved exposure and lock
                state.release_exposure(reserved_exposure);
                state.release_execution(opp.market_id().as_str());
            }
        }
        RiskCheckResult::Rejected(error) => {
            // Record rejected opportunity
            stats.record_opportunity(&RecordedOpportunity {
                strategy: opp.strategy().to_string(),
                market_ids: vec![opp.market_id().to_string()],
                edge: opp.edge(),
                expected_profit: opp.expected_profit(),
                executed: false,
                rejected_reason: Some(format!("{error}")),
            });

            // Release the lock on rejection
            state.release_execution(opp.market_id().as_str());
            notifiers.notify_all(Event::RiskRejected(RiskEvent::new(
                opp.market_id().as_str(),
                &error,
            )));
        }
    }
}

/// Get the maximum slippage across all legs.
/// Returns None if prices cannot be determined (books not in cache or empty).
pub(crate) fn get_max_slippage(
    opportunity: &Opportunity,
    cache: &OrderBookCache,
) -> Option<Decimal> {
    let mut max_slippage = Decimal::ZERO;

    for leg in opportunity.legs() {
        let book = cache.get(leg.token_id())?;
        let current_price = book.best_ask()?.price();
        let expected_price = leg.ask_price();

        if expected_price == Decimal::ZERO {
            return None;
        }

        let slippage = ((current_price - expected_price).abs()) / expected_price;
        max_slippage = max_slippage.max(slippage);
    }

    Some(max_slippage)
}
