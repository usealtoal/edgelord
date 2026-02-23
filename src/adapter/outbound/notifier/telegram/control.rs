//! Telegram command execution against runtime app state.

use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use chrono::Utc;
use parking_lot::RwLock;

use crate::port::{
    inbound::runtime::RuntimeClusterView, inbound::runtime::RuntimeState,
    outbound::exchange::PoolStats, outbound::stats::StatsRecorder,
};

mod dispatch;
mod mutate;
mod render;
mod runtime;

/// Runtime statistics updated by the orchestrator.
///
/// These values are updated periodically and read by Telegram commands.
#[derive(Default)]
pub struct RuntimeStats {
    /// Connection pool statistics.
    pool_stats: RwLock<Option<PoolStats>>,
    /// Number of subscribed markets.
    market_count: AtomicUsize,
    /// Number of subscribed tokens.
    token_count: AtomicUsize,
    /// Cluster view for relation lookups.
    cluster_view: RwLock<Option<Arc<dyn RuntimeClusterView>>>,
}

/// Runtime command executor for Telegram control commands.
#[derive(Clone)]
pub struct TelegramControl {
    state: Arc<dyn RuntimeState>,
    stats_recorder: Option<Arc<dyn StatsRecorder>>,
    runtime_stats: Option<Arc<RuntimeStats>>,
    started_at: chrono::DateTime<Utc>,
    /// Maximum positions to display in /positions command.
    position_display_limit: usize,
}

/// Default position display limit if not specified.
const DEFAULT_POSITION_DISPLAY_LIMIT: usize = 10;

fn format_uptime(started_at: chrono::DateTime<Utc>) -> String {
    runtime::format_uptime(started_at)
}

#[cfg(test)]
mod tests;
