use std::sync::atomic::Ordering;
use std::sync::Arc;

use chrono::Utc;

use crate::port::{inbound::runtime::RuntimeClusterView, outbound::exchange::PoolStats};

use super::RuntimeStats;

impl RuntimeStats {
    /// Create a new runtime stats container.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Update pool statistics.
    pub fn update_pool_stats(&self, stats: PoolStats) {
        *self.pool_stats.write() = Some(stats);
    }

    /// Update market and token counts.
    pub fn update_market_counts(&self, markets: usize, tokens: usize) {
        self.market_count.store(markets, Ordering::Relaxed);
        self.token_count.store(tokens, Ordering::Relaxed);
    }

    /// Get current pool stats.
    #[must_use]
    pub fn pool_stats(&self) -> Option<PoolStats> {
        self.pool_stats.read().clone()
    }

    /// Get market count.
    #[must_use]
    pub fn market_count(&self) -> usize {
        self.market_count.load(Ordering::Relaxed)
    }

    /// Get token count.
    #[must_use]
    pub fn token_count(&self) -> usize {
        self.token_count.load(Ordering::Relaxed)
    }

    /// Set cluster view for relation lookups.
    pub fn set_cluster_cache(&self, view: Arc<dyn RuntimeClusterView>) {
        *self.cluster_view.write() = Some(view);
    }

    /// Get cluster view.
    #[must_use]
    pub fn cluster_view(&self) -> Option<Arc<dyn RuntimeClusterView>> {
        self.cluster_view.read().clone()
    }
}

pub(super) fn format_uptime(started_at: chrono::DateTime<Utc>) -> String {
    let elapsed = Utc::now() - started_at;
    let total_seconds = elapsed.num_seconds().max(0);
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    format!("{hours:02}:{minutes:02}:{seconds:02}")
}
