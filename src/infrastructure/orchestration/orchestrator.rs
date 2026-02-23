//! Infrastructure orchestration façade.

use crate::application::orchestration::handler::handle_market_event;
use crate::port::outbound::exchange::MarketEvent;

pub use super::context::EventProcessingContext;
pub use super::health::{health_check, HealthCheck, HealthReport, HealthStatus};
pub use super::runtime::run_with_shutdown;

/// Main application orchestrator.
pub struct Orchestrator;

/// Process a single market event through the orchestrator pipeline.
pub fn process_market_event(event: MarketEvent, context: EventProcessingContext<'_>) {
    handle_market_event(event, context.into_handler_context());
}
