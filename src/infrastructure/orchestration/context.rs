//! Infrastructure-level event processing context.

use std::sync::Arc;

use crate::application::cache::book::BookCache;
use crate::application::orchestration::handler::MarketEventHandlingContext;
use crate::application::position::manager::PositionManager;
use crate::application::risk::manager::RiskManager;
use crate::application::state::AppState;
use crate::domain::market::MarketRegistry;
use crate::port::inbound::strategy::StrategyEngine;
use crate::port::outbound::exchange::ArbitrageExecutor;
use crate::port::outbound::notifier::NotifierRegistry;
use crate::port::outbound::stats::StatsRecorder;

/// Context used by infrastructure entrypoints to process one market event.
pub struct EventProcessingContext<'a> {
    pub cache: &'a BookCache,
    pub registry: &'a MarketRegistry,
    pub strategies: &'a dyn StrategyEngine,
    pub executor: Option<Arc<dyn ArbitrageExecutor + Send + Sync>>,
    pub risk_manager: &'a RiskManager,
    pub notifiers: &'a Arc<NotifierRegistry>,
    pub state: &'a Arc<AppState>,
    pub stats: &'a Arc<dyn StatsRecorder>,
    pub position_manager: &'a Arc<PositionManager>,
    pub dry_run: bool,
}

impl<'a> EventProcessingContext<'a> {
    pub(crate) fn into_handler_context(self) -> MarketEventHandlingContext<'a> {
        MarketEventHandlingContext {
            cache: self.cache,
            registry: self.registry,
            strategies: self.strategies,
            executor: self.executor,
            risk_manager: self.risk_manager,
            notifiers: self.notifiers,
            state: self.state,
            stats: self.stats,
            position_manager: self.position_manager,
            dry_run: self.dry_run,
        }
    }
}
