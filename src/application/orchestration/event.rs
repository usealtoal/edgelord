//! Market-event flow for orchestration.

use std::time::Instant;

use rust_decimal::Decimal;
use tracing::{debug, info, warn};

use super::context::MarketDetectionContext;
use super::handler::handle_opportunity;
use super::handler::MarketEventHandlingContext;
use crate::application::position::manager::{CloseReason, PositionManager};
use crate::port::outbound::exchange::MarketEvent;

/// Handle incoming market events from the data stream.
pub(crate) fn handle_market_event(event: MarketEvent, context: MarketEventHandlingContext<'_>) {
    let start = Instant::now();

    match event {
        MarketEvent::BookSnapshot { token_id, book } => {
            context.cache.update(book);

            if let Some(market) = context.registry.get_by_token(&token_id) {
                let ctx = MarketDetectionContext::new(market, context.cache);
                let opportunities = context.strategies.detect_opportunities(&ctx);

                debug!(
                    market_id = %market.market_id(),
                    opportunities_found = opportunities.len(),
                    "Strategy detection complete (snapshot)"
                );

                for opp in opportunities {
                    handle_opportunity(opp, context.opportunity_context());
                }
            }

            let elapsed = start.elapsed();
            context.stats.record_latency(elapsed.as_millis() as u32);
        }
        MarketEvent::BookDelta { token_id, book } => {
            context.cache.update(book);

            if let Some(market) = context.registry.get_by_token(&token_id) {
                let ctx = MarketDetectionContext::new(market, context.cache);
                let opportunities = context.strategies.detect_opportunities(&ctx);

                debug!(
                    market_id = %market.market_id(),
                    opportunities_found = opportunities.len(),
                    "Strategy detection complete (delta)"
                );

                for opp in opportunities {
                    handle_opportunity(opp, context.opportunity_context());
                }
            }

            let elapsed = start.elapsed();
            context.stats.record_latency(elapsed.as_millis() as u32);
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

            let mut tracker = context.state.positions_mut();
            let total_pnl = context.position_manager.close_all_for_market(
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
