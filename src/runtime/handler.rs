//! Event and opportunity handling.

use std::sync::Arc;
use std::time::Instant;

use rust_decimal::Decimal;
use tracing::{debug, info, warn};

use super::execution::spawn_execution;
use super::state::AppState;
use crate::adapters::notifiers::NotifierRegistry;
use crate::adapters::notifiers::{Event, OpportunityEvent, RiskEvent};
use crate::adapters::position::{CloseReason, PositionManager};
use crate::adapters::risk::{RiskCheckResult, RiskManager};
use crate::adapters::statistics::{RecordedOpportunity, StatsRecorder};
use crate::adapters::strategies::DetectionContext;
use crate::adapters::strategies::StrategyRegistry;
use crate::domain::{MarketRegistry, Opportunity};
use crate::error::RiskError;
use crate::runtime::cache::OrderBookCache;
use crate::runtime::exchange::{ArbitrageExecutor, MarketEvent};

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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rust_decimal_macros::dec;

    use super::*;
    use crate::adapters::notifiers::NotifierRegistry;
    use crate::adapters::risk::RiskManager;
    use crate::adapters::statistics;
    use crate::adapters::strategies::StrategyRegistry;
    use crate::domain::{
        Market, MarketId, Opportunity, OpportunityLeg, OrderBook, Outcome, PriceLevel, TokenId,
    };
    use crate::runtime::cache::OrderBookCache;
    use crate::runtime::{AppState, RiskLimits};

    fn make_order_book(token_id: &str, bid: Decimal, ask: Decimal) -> OrderBook {
        OrderBook::with_levels(
            TokenId::from(token_id),
            vec![PriceLevel::new(bid, Decimal::new(100, 0))],
            vec![PriceLevel::new(ask, Decimal::new(100, 0))],
        )
    }

    fn make_binary_market(
        id: &str,
        question: &str,
        yes_token: &str,
        no_token: &str,
        payout: Decimal,
    ) -> Market {
        let outcomes = vec![
            Outcome::new(TokenId::from(yes_token), "Yes"),
            Outcome::new(TokenId::from(no_token), "No"),
        ];
        Market::new(MarketId::from(id), question, outcomes, payout)
    }

    fn make_registry(markets: Vec<Market>) -> MarketRegistry {
        let mut registry = MarketRegistry::new();
        for market in markets {
            registry.add(market);
        }
        registry
    }

    fn make_test_opportunity() -> Opportunity {
        Opportunity::new(
            MarketId::from("test-market"),
            "Will it rain?",
            vec![
                OpportunityLeg::new(TokenId::from("yes-token"), dec!(0.40)),
                OpportunityLeg::new(TokenId::from("no-token"), dec!(0.50)),
            ],
            dec!(100),
            dec!(1.00),
        )
    }

    // ========== get_max_slippage tests ==========

    #[test]
    fn get_max_slippage_returns_none_when_token_not_in_cache() {
        let cache = OrderBookCache::new();
        let opp = make_test_opportunity();
        let result = get_max_slippage(&opp, &cache);
        assert!(result.is_none());
    }

    #[test]
    fn get_max_slippage_returns_none_when_book_has_no_asks() {
        let cache = OrderBookCache::new();
        // Create order book with bids but no asks
        let empty_ask_book = OrderBook::with_levels(
            TokenId::from("yes-token"),
            vec![PriceLevel::new(dec!(0.40), dec!(100))],
            vec![], // no asks
        );
        cache.update(empty_ask_book);
        cache.update(make_order_book("no-token", dec!(0.49), dec!(0.50)));

        let opp = make_test_opportunity();
        let result = get_max_slippage(&opp, &cache);
        assert!(result.is_none());
    }

    #[test]
    fn get_max_slippage_returns_none_when_expected_price_is_zero() {
        let cache = OrderBookCache::new();
        cache.update(make_order_book("yes-token", dec!(0.39), dec!(0.40)));
        cache.update(make_order_book("no-token", dec!(0.49), dec!(0.50)));

        // Create opportunity with zero price leg
        let opp = Opportunity::new(
            MarketId::from("test-market"),
            "Zero price test?",
            vec![
                OpportunityLeg::new(TokenId::from("yes-token"), dec!(0.00)),
                OpportunityLeg::new(TokenId::from("no-token"), dec!(0.50)),
            ],
            dec!(100),
            dec!(1.00),
        );

        let result = get_max_slippage(&opp, &cache);
        assert!(result.is_none());
    }

    #[test]
    fn get_max_slippage_returns_zero_when_prices_match() {
        let cache = OrderBookCache::new();
        cache.update(make_order_book("yes-token", dec!(0.39), dec!(0.40)));
        cache.update(make_order_book("no-token", dec!(0.49), dec!(0.50)));

        let opp = make_test_opportunity();
        let result = get_max_slippage(&opp, &cache);
        assert_eq!(result, Some(dec!(0)));
    }

    #[test]
    fn get_max_slippage_calculates_slippage_when_price_increased() {
        let cache = OrderBookCache::new();
        // Price increased from 0.40 to 0.42 = 5% slippage
        cache.update(make_order_book("yes-token", dec!(0.41), dec!(0.42)));
        cache.update(make_order_book("no-token", dec!(0.49), dec!(0.50)));

        let opp = make_test_opportunity();
        let result = get_max_slippage(&opp, &cache);
        assert_eq!(result, Some(dec!(0.05))); // (0.42 - 0.40) / 0.40 = 0.05
    }

    #[test]
    fn get_max_slippage_calculates_slippage_when_price_decreased() {
        let cache = OrderBookCache::new();
        // Price decreased from 0.40 to 0.38 = 5% slippage (absolute)
        cache.update(make_order_book("yes-token", dec!(0.37), dec!(0.38)));
        cache.update(make_order_book("no-token", dec!(0.49), dec!(0.50)));

        let opp = make_test_opportunity();
        let result = get_max_slippage(&opp, &cache);
        assert_eq!(result, Some(dec!(0.05))); // |0.38 - 0.40| / 0.40 = 0.05
    }

    #[test]
    fn get_max_slippage_returns_max_across_legs() {
        let cache = OrderBookCache::new();
        // Yes: 5% slippage, No: 10% slippage
        cache.update(make_order_book("yes-token", dec!(0.41), dec!(0.42)));
        cache.update(make_order_book("no-token", dec!(0.54), dec!(0.55)));

        let opp = make_test_opportunity();
        let result = get_max_slippage(&opp, &cache);
        assert_eq!(result, Some(dec!(0.10))); // max(0.05, 0.10) = 0.10
    }

    // ========== handle_opportunity tests ==========

    #[test]
    fn handle_opportunity_skips_when_execution_already_locked() {
        let opp = make_test_opportunity();
        let state = Arc::new(AppState::default());
        let notifiers = Arc::new(NotifierRegistry::new());
        let risk_manager = RiskManager::new(Arc::clone(&state));
        let db_pool = crate::adapters::stores::db::create_pool("sqlite://:memory:").unwrap();
        let stats = statistics::create_recorder(db_pool);
        let cache = OrderBookCache::new();

        // Lock the market first
        state.try_lock_execution("test-market");

        // Try to handle opportunity - should return early
        handle_opportunity(
            opp,
            None,
            &risk_manager,
            &notifiers,
            &state,
            &stats,
            &cache,
            true,
        );

        // No notifications should have been sent (no OpportunityDetected)
        // The function returns early before notifying
    }

    #[test]
    fn handle_opportunity_rejects_high_slippage() {
        let state = Arc::new(AppState::new(RiskLimits {
            max_slippage: dec!(0.01), // 1% max slippage
            ..Default::default()
        }));
        let notifiers = Arc::new(NotifierRegistry::new());
        let risk_manager = RiskManager::new(Arc::clone(&state));
        let db_pool = crate::adapters::stores::db::create_pool("sqlite://:memory:").unwrap();
        let stats = statistics::create_recorder(db_pool);
        let cache = OrderBookCache::new();

        // Set up cache with 5% slippage
        cache.update(make_order_book("yes-token", dec!(0.41), dec!(0.42)));
        cache.update(make_order_book("no-token", dec!(0.49), dec!(0.50)));

        let opp = make_test_opportunity();

        handle_opportunity(
            opp,
            None,
            &risk_manager,
            &notifiers,
            &state,
            &stats,
            &cache,
            true,
        );

        // Execution lock should be released
        assert!(
            state.try_lock_execution("test-market"),
            "Lock should be released after slippage rejection"
        );
    }

    #[test]
    fn handle_opportunity_releases_lock_on_risk_rejection() {
        let state = Arc::new(AppState::new(RiskLimits {
            min_profit_threshold: dec!(100), // Require $100 profit minimum
            ..Default::default()
        }));
        let notifiers = Arc::new(NotifierRegistry::new());
        let risk_manager = RiskManager::new(Arc::clone(&state));
        let db_pool = crate::adapters::stores::db::create_pool("sqlite://:memory:").unwrap();
        let stats = statistics::create_recorder(db_pool);
        let cache = OrderBookCache::new();

        cache.update(make_order_book("yes-token", dec!(0.39), dec!(0.40)));
        cache.update(make_order_book("no-token", dec!(0.49), dec!(0.50)));

        let opp = make_test_opportunity(); // Expected profit = $10, below threshold

        handle_opportunity(
            opp,
            None,
            &risk_manager,
            &notifiers,
            &state,
            &stats,
            &cache,
            true,
        );

        // Lock should be released after risk rejection
        assert!(
            state.try_lock_execution("test-market"),
            "Lock should be released after risk rejection"
        );
    }

    #[test]
    fn handle_opportunity_dry_run_releases_lock_and_exposure() {
        let state = Arc::new(AppState::default());
        let notifiers = Arc::new(NotifierRegistry::new());
        let risk_manager = RiskManager::new(Arc::clone(&state));
        let db_pool = crate::adapters::stores::db::create_pool("sqlite://:memory:").unwrap();
        let stats = statistics::create_recorder(db_pool);
        let cache = OrderBookCache::new();

        cache.update(make_order_book("yes-token", dec!(0.39), dec!(0.40)));
        cache.update(make_order_book("no-token", dec!(0.49), dec!(0.50)));

        let opp = make_test_opportunity();

        handle_opportunity(
            opp,
            None,
            &risk_manager,
            &notifiers,
            &state,
            &stats,
            &cache,
            true, // dry_run = true
        );

        // Lock should be released
        assert!(
            state.try_lock_execution("test-market"),
            "Lock should be released after dry run"
        );

        // Exposure should be released (back to zero)
        assert_eq!(
            state.pending_exposure(),
            dec!(0),
            "Pending exposure should be zero after dry run"
        );
    }

    #[test]
    fn handle_opportunity_no_executor_releases_lock_and_exposure() {
        let state = Arc::new(AppState::default());
        let notifiers = Arc::new(NotifierRegistry::new());
        let risk_manager = RiskManager::new(Arc::clone(&state));
        let db_pool = crate::adapters::stores::db::create_pool("sqlite://:memory:").unwrap();
        let stats = statistics::create_recorder(db_pool);
        let cache = OrderBookCache::new();

        cache.update(make_order_book("yes-token", dec!(0.39), dec!(0.40)));
        cache.update(make_order_book("no-token", dec!(0.49), dec!(0.50)));

        let opp = make_test_opportunity();

        handle_opportunity(
            opp,
            None, // No executor
            &risk_manager,
            &notifiers,
            &state,
            &stats,
            &cache,
            false, // dry_run = false
        );

        // Lock should be released
        assert!(
            state.try_lock_execution("test-market"),
            "Lock should be released when no executor"
        );
    }

    // ========== handle_market_event tests ==========

    #[test]
    fn handle_market_event_updates_cache_on_snapshot() {
        let cache = Arc::new(OrderBookCache::new());
        let registry = Arc::new(make_registry(vec![make_binary_market(
            "market-1",
            "Test?",
            "yes-1",
            "no-1",
            dec!(1.00),
        )]));
        let strategies = StrategyRegistry::new();
        let state = Arc::new(AppState::default());
        let notifiers = Arc::new(NotifierRegistry::new());
        let risk_manager = Arc::new(RiskManager::new(Arc::clone(&state)));
        let db_pool = crate::adapters::stores::db::create_pool("sqlite://:memory:").unwrap();
        let stats = statistics::create_recorder(db_pool);
        let position_manager = Arc::new(crate::adapters::position::PositionManager::new(
            Arc::clone(&stats),
        ));

        let book = make_order_book("yes-1", dec!(0.40), dec!(0.42));

        handle_market_event(
            MarketEvent::OrderBookSnapshot {
                token_id: TokenId::from("yes-1"),
                book,
            },
            &cache,
            &registry,
            &strategies,
            None,
            &risk_manager,
            &notifiers,
            &state,
            &stats,
            &position_manager,
            true,
        );

        // Cache should have the order book
        let cached = cache.get(&TokenId::from("yes-1"));
        assert!(cached.is_some(), "Order book should be in cache");
        assert_eq!(
            cached.unwrap().best_ask().unwrap().price(),
            dec!(0.42),
            "Cached ask price should match"
        );
    }

    #[test]
    fn handle_market_event_updates_cache_on_delta() {
        let cache = Arc::new(OrderBookCache::new());
        let registry = Arc::new(make_registry(vec![make_binary_market(
            "market-1",
            "Test?",
            "yes-1",
            "no-1",
            dec!(1.00),
        )]));
        let strategies = StrategyRegistry::new();
        let state = Arc::new(AppState::default());
        let notifiers = Arc::new(NotifierRegistry::new());
        let risk_manager = Arc::new(RiskManager::new(Arc::clone(&state)));
        let db_pool = crate::adapters::stores::db::create_pool("sqlite://:memory:").unwrap();
        let stats = statistics::create_recorder(db_pool);
        let position_manager = Arc::new(crate::adapters::position::PositionManager::new(
            Arc::clone(&stats),
        ));

        let book = make_order_book("yes-1", dec!(0.40), dec!(0.42));

        handle_market_event(
            MarketEvent::OrderBookDelta {
                token_id: TokenId::from("yes-1"),
                book,
            },
            &cache,
            &registry,
            &strategies,
            None,
            &risk_manager,
            &notifiers,
            &state,
            &stats,
            &position_manager,
            true,
        );

        // Cache should have the order book
        let cached = cache.get(&TokenId::from("yes-1"));
        assert!(cached.is_some(), "Order book should be in cache");
    }

    #[test]
    fn handle_market_event_settles_positions() {
        let cache = Arc::new(OrderBookCache::new());
        let registry = Arc::new(MarketRegistry::new());
        let strategies = StrategyRegistry::new();
        let state = Arc::new(AppState::default());
        let notifiers = Arc::new(NotifierRegistry::new());
        let risk_manager = Arc::new(RiskManager::new(Arc::clone(&state)));
        let db_pool = crate::adapters::stores::db::create_pool("sqlite://:memory:").unwrap();
        let stats = statistics::create_recorder(db_pool);
        let position_manager = Arc::new(crate::adapters::position::PositionManager::new(
            Arc::clone(&stats),
        ));

        // Add a position to close
        {
            use crate::domain::{Position, PositionLeg, PositionStatus};
            let mut positions = state.positions_mut();
            let legs = vec![
                PositionLeg::new(TokenId::from("yes-1"), dec!(100), dec!(0.40)),
                PositionLeg::new(TokenId::from("no-1"), dec!(100), dec!(0.50)),
            ];
            let position = Position::new(
                positions.next_id(),
                MarketId::from("settled-market"),
                legs,
                dec!(90),
                dec!(100),
                chrono::Utc::now(),
                PositionStatus::Open,
            );
            positions.add(position);
        }

        // Verify position exists
        assert_eq!(state.positions().all().count(), 1);

        handle_market_event(
            MarketEvent::MarketSettled {
                market_id: MarketId::from("settled-market"),
                winning_outcome: "Yes".to_string(),
                payout_per_share: dec!(1.00),
            },
            &cache,
            &registry,
            &strategies,
            None,
            &risk_manager,
            &notifiers,
            &state,
            &stats,
            &position_manager,
            true,
        );

        // Position should be closed
        let positions = state.positions();
        let open_positions: Vec<_> = positions.all().collect();
        assert!(
            open_positions.is_empty()
                || open_positions
                    .iter()
                    .all(|p| !matches!(p.status(), crate::domain::PositionStatus::Open)),
            "Position should be closed after settlement"
        );
    }

    #[test]
    fn handle_market_event_connected_does_not_panic() {
        let cache = Arc::new(OrderBookCache::new());
        let registry = Arc::new(MarketRegistry::new());
        let strategies = StrategyRegistry::new();
        let state = Arc::new(AppState::default());
        let notifiers = Arc::new(NotifierRegistry::new());
        let risk_manager = Arc::new(RiskManager::new(Arc::clone(&state)));
        let db_pool = crate::adapters::stores::db::create_pool("sqlite://:memory:").unwrap();
        let stats = statistics::create_recorder(db_pool);
        let position_manager = Arc::new(crate::adapters::position::PositionManager::new(
            Arc::clone(&stats),
        ));

        // Should not panic
        handle_market_event(
            MarketEvent::Connected,
            &cache,
            &registry,
            &strategies,
            None,
            &risk_manager,
            &notifiers,
            &state,
            &stats,
            &position_manager,
            true,
        );
    }

    #[test]
    fn handle_market_event_disconnected_does_not_panic() {
        let cache = Arc::new(OrderBookCache::new());
        let registry = Arc::new(MarketRegistry::new());
        let strategies = StrategyRegistry::new();
        let state = Arc::new(AppState::default());
        let notifiers = Arc::new(NotifierRegistry::new());
        let risk_manager = Arc::new(RiskManager::new(Arc::clone(&state)));
        let db_pool = crate::adapters::stores::db::create_pool("sqlite://:memory:").unwrap();
        let stats = statistics::create_recorder(db_pool);
        let position_manager = Arc::new(crate::adapters::position::PositionManager::new(
            Arc::clone(&stats),
        ));

        // Should not panic
        handle_market_event(
            MarketEvent::Disconnected {
                reason: "Test disconnect".to_string(),
            },
            &cache,
            &registry,
            &strategies,
            None,
            &risk_manager,
            &notifiers,
            &state,
            &stats,
            &position_manager,
            true,
        );
    }

    #[test]
    fn handle_market_event_ignores_unknown_token() {
        let cache = Arc::new(OrderBookCache::new());
        let registry = Arc::new(MarketRegistry::new()); // Empty registry
        let strategies = StrategyRegistry::new();
        let state = Arc::new(AppState::default());
        let notifiers = Arc::new(NotifierRegistry::new());
        let risk_manager = Arc::new(RiskManager::new(Arc::clone(&state)));
        let db_pool = crate::adapters::stores::db::create_pool("sqlite://:memory:").unwrap();
        let stats = statistics::create_recorder(db_pool);
        let position_manager = Arc::new(crate::adapters::position::PositionManager::new(
            Arc::clone(&stats),
        ));

        let book = make_order_book("unknown-token", dec!(0.40), dec!(0.42));

        // Should not panic, just update cache and skip strategy detection
        handle_market_event(
            MarketEvent::OrderBookSnapshot {
                token_id: TokenId::from("unknown-token"),
                book,
            },
            &cache,
            &registry,
            &strategies,
            None,
            &risk_manager,
            &notifiers,
            &state,
            &stats,
            &position_manager,
            true,
        );

        // Cache should still be updated
        assert!(cache.get(&TokenId::from("unknown-token")).is_some());
    }
}
