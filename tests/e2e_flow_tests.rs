mod harness;
mod support;

use std::sync::Arc;

use edgelord::adapter::outbound::sqlite::stats_recorder::create_recorder;
use edgelord::application::cache::book::BookCache;
use edgelord::application::position::manager::PositionManager;
use edgelord::application::risk::manager::RiskManager;
use edgelord::application::state::{AppState, RiskLimits};
use edgelord::application::strategy::registry::StrategyRegistry;
use edgelord::application::strategy::single_condition::{
    SingleConditionConfig, SingleConditionStrategy,
};
use edgelord::domain::id::TokenId;
use edgelord::infrastructure::orchestration::orchestrator::{
    process_market_event, EventProcessingContext,
};
use edgelord::port::outbound::exchange::MarketEvent;
use edgelord::port::outbound::notifier::NotifierRegistry;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[test]
fn harness_scripted_stream_accepts_token_subscription() {
    let mut stream = harness::scripted_stream::ScriptedMarketDataStream::default();
    stream.push_connected();
    stream.subscribe_tokens(&[TokenId::new("token-1"), TokenId::new("token-2")]);

    assert_eq!(stream.subscriptions().len(), 1);
    assert_eq!(stream.subscriptions()[0].len(), 2);
}

#[test]
fn e2e_ingest_detect_persist_notify() {
    let market = support::market::make_binary_market(
        "market-1",
        "Will this event happen?",
        "yes-token",
        "no-token",
        dec!(1.00),
    );
    let registry = support::registry::make_registry(vec![market]);

    let mut strategies = StrategyRegistry::new();
    strategies.register(Box::new(SingleConditionStrategy::new(
        SingleConditionConfig::default(),
    )));

    let cache = BookCache::new();
    let state = Arc::new(AppState::new(RiskLimits {
        min_profit_threshold: Decimal::ZERO,
        ..Default::default()
    }));
    let risk_manager = RiskManager::new(Arc::clone(&state));

    let db = harness::temp_db::TempDb::create("e2e-flow");
    let stats = create_recorder(db.pool().clone());
    let position_manager = Arc::new(PositionManager::new(Arc::clone(&stats)));

    let mut notifier_registry = NotifierRegistry::new();
    let notifier = harness::recording_notifier::RecordingNotifier::new();
    notifier_registry.register(Box::new(notifier.clone()));
    let notifiers = Arc::new(notifier_registry);

    let yes_book = support::book::make_book("yes-token", dec!(0.39), dec!(0.40));
    let no_book = support::book::make_book("no-token", dec!(0.49), dec!(0.50));

    process_market_event(
        MarketEvent::BookSnapshot {
            token_id: TokenId::new("yes-token"),
            book: yes_book,
        },
        EventProcessingContext {
            cache: &cache,
            registry: &registry,
            strategies: &strategies,
            executor: None,
            risk_manager: &risk_manager,
            notifiers: &notifiers,
            state: &state,
            stats: &stats,
            position_manager: &position_manager,
            dry_run: true,
        },
    );

    process_market_event(
        MarketEvent::BookSnapshot {
            token_id: TokenId::new("no-token"),
            book: no_book,
        },
        EventProcessingContext {
            cache: &cache,
            registry: &registry,
            strategies: &strategies,
            executor: None,
            risk_manager: &risk_manager,
            notifiers: &notifiers,
            state: &state,
            stats: &stats,
            position_manager: &position_manager,
            dry_run: true,
        },
    );

    let summary = stats.get_today();
    assert_eq!(summary.opportunities_detected, 1);
    assert_eq!(notifier.len(), 1);
}
