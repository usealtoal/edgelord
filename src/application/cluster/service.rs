//! Cluster detection service for combinatorial arbitrage.
//!
//! Background service that monitors order book updates and runs Frank-Wolfe
//! detection on clusters of related markets. Operates independently of
//! per-market detection for better scalability.
//!
//! # Architecture
//!
//! ```text
//! BookCache --(broadcast)--> ClusterDetectionService
//!                                        |
//!                                        +-- tracks dirty clusters
//!                                        +-- debounces detection
//!                                        +-- ClusterDetector::detect()
//!                                                   |
//!                                                   v
//!                                           ClusterOpportunity
//! ```

use super::detector::ClusterDetector;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use rust_decimal::Decimal;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, info, trace, warn};

use crate::application::cache::book::{BookCache, BookUpdate};
use crate::application::cache::cluster::ClusterCache;
use crate::domain::{id::MarketId, id::TokenId, market::MarketRegistry, opportunity::Opportunity};
use crate::port::outbound::solver::ProjectionSolver;

/// Configuration for the cluster detection service.
#[derive(Debug, Clone)]
pub struct ClusterDetectionConfig {
    /// Debounce interval in milliseconds between detection cycles.
    pub debounce_ms: u64,
    /// Minimum arbitrage gap required to report an opportunity.
    pub min_gap: Decimal,
    /// Maximum number of clusters to process per detection cycle.
    pub max_clusters_per_cycle: usize,
}

impl Default for ClusterDetectionConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 100,
            min_gap: Decimal::new(2, 2), // 0.02 = 2%
            max_clusters_per_cycle: 50,
        }
    }
}

/// Opportunity discovered by cluster detection.
#[derive(Debug, Clone)]
pub struct ClusterOpportunity {
    /// ID of the cluster where the opportunity was found.
    pub cluster_id: String,
    /// Market IDs involved in the opportunity.
    pub markets: Vec<MarketId>,
    /// Arbitrage gap (Bregman divergence from fair prices).
    pub gap: Decimal,
    /// Full opportunity details for execution.
    pub opportunity: Opportunity,
}

/// Handle for controlling the cluster detection service lifecycle.
pub struct ClusterDetectionHandle {
    /// Channel for sending shutdown signal.
    shutdown_tx: mpsc::Sender<()>,
}

impl ClusterDetectionHandle {
    /// Signal the service to shut down gracefully.
    pub async fn shutdown(&self) {
        let _ = self.shutdown_tx.send(()).await;
    }
}

/// Background service for cluster-based arbitrage detection.
///
/// Monitors order book updates via broadcast channel, tracks which clusters
/// have pending updates, and periodically runs detection on dirty clusters.
/// Uses [`ClusterDetector`] for the actual Frank-Wolfe projection logic.
pub struct ClusterDetectionService {
    /// Service configuration.
    config: ClusterDetectionConfig,
    /// Order book cache for price lookups.
    order_book_cache: Arc<BookCache>,
    /// Cluster cache for relation lookups.
    cluster_cache: Arc<ClusterCache>,
    /// Market registry for resolving metadata.
    registry: Arc<MarketRegistry>,
    /// Detector instance for running Frank-Wolfe.
    detector: ClusterDetector,
    /// Mapping from token ID to market ID for efficient reverse lookup.
    token_to_market: HashMap<TokenId, MarketId>,
    /// Set of cluster IDs with pending updates.
    dirty_clusters: Arc<RwLock<HashSet<String>>>,
}

impl ClusterDetectionService {
    /// Create a new cluster detection service with the given dependencies.
    pub fn new(
        config: ClusterDetectionConfig,
        order_book_cache: Arc<BookCache>,
        cluster_cache: Arc<ClusterCache>,
        registry: Arc<MarketRegistry>,
        projection_solver: Arc<dyn ProjectionSolver>,
    ) -> Self {
        // Build token -> market mapping for efficient lookups
        let token_to_market: HashMap<TokenId, MarketId> = registry
            .markets()
            .iter()
            .flat_map(|m| {
                m.outcomes()
                    .iter()
                    .map(|o| (o.token_id().clone(), m.market_id().clone()))
            })
            .collect();

        let detector = ClusterDetector::new(config.clone(), projection_solver);

        Self {
            config,
            order_book_cache,
            cluster_cache,
            registry,
            detector,
            token_to_market,
            dirty_clusters: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Start the background detection service.
    ///
    /// Spawns an async task that monitors order book updates and runs
    /// detection periodically. Returns a handle for lifecycle control
    /// and a channel for receiving discovered opportunities.
    pub fn start(
        self,
        mut update_rx: broadcast::Receiver<BookUpdate>,
    ) -> (ClusterDetectionHandle, mpsc::Receiver<ClusterOpportunity>) {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        let (opportunity_tx, opportunity_rx) = mpsc::channel::<ClusterOpportunity>(64);

        let service = Arc::new(self);
        let debounce_duration = Duration::from_millis(service.config.debounce_ms);

        tokio::spawn(async move {
            let mut last_detection = Instant::now();

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        info!("Cluster detection service shutting down");
                        break;
                    }

                    update = update_rx.recv() => {
                        match update {
                            Ok(u) => service.handle_update(&u),
                            Err(broadcast::error::RecvError::Lagged(n)) => {
                                warn!(skipped = n, "Cluster detection lagged, some updates missed");
                            }
                            Err(broadcast::error::RecvError::Closed) => {
                                info!("Order book update channel closed");
                                break;
                            }
                        }
                    }

                    _ = tokio::time::sleep(debounce_duration) => {
                        if last_detection.elapsed() >= debounce_duration {
                            let opportunities = service.run_detection();
                            for opp in opportunities {
                                if opportunity_tx.send(opp).await.is_err() {
                                    debug!("Opportunity receiver dropped, stopping service");
                                    return;
                                }
                            }
                            last_detection = Instant::now();
                        }
                    }
                }
            }
        });

        (ClusterDetectionHandle { shutdown_tx }, opportunity_rx)
    }

    /// Process an order book update by marking affected clusters as dirty.
    fn handle_update(&self, update: &BookUpdate) {
        let Some(market_id) = self.token_to_market.get(&update.token_id) else {
            return;
        };

        if let Some(cluster) = self.cluster_cache.get_for_market(market_id) {
            self.dirty_clusters.write().insert(cluster.id.to_string());
            trace!(market = %market_id, cluster = %cluster.id, "Marked cluster dirty");
        }
    }

    /// Run detection on all dirty clusters and return discovered opportunities.
    fn run_detection(&self) -> Vec<ClusterOpportunity> {
        // Atomically grab and clear dirty clusters
        let dirty: Vec<String> = {
            let mut clusters = self.dirty_clusters.write();
            let dirty: Vec<_> = clusters
                .iter()
                .take(self.config.max_clusters_per_cycle)
                .cloned()
                .collect();
            for id in &dirty {
                clusters.remove(id);
            }
            dirty
        };

        if dirty.is_empty() {
            return Vec::new();
        }

        debug!(count = dirty.len(), "Running detection on dirty clusters");

        let mut opportunities = Vec::new();
        let mut errors = 0;

        for cluster_id in dirty {
            match self.detect_cluster(&cluster_id) {
                Ok(Some(opp)) => opportunities.push(opp),
                Ok(None) => {} // Gap below threshold, not an error
                Err(e) => {
                    debug!(cluster = %cluster_id, error = %e, "Detection failed");
                    errors += 1;
                }
            }
        }

        if errors > 0 {
            warn!(errors = errors, "Some cluster detections failed");
        }

        opportunities
    }

    /// Run detection on a single cluster by ID.
    fn detect_cluster(&self, cluster_id: &str) -> crate::error::Result<Option<ClusterOpportunity>> {
        let cluster = self
            .cluster_cache
            .all_clusters()
            .into_iter()
            .find(|c| c.id.to_string() == cluster_id)
            .ok_or_else(|| {
                crate::error::Error::Parse(format!("Cluster not found: {cluster_id}"))
            })?;

        let book_lookup = |token_id: &TokenId| self.order_book_cache.get(token_id);
        self.detector.detect(&cluster, &book_lookup, &self.registry)
    }

    /// Return the number of clusters with pending updates.
    ///
    /// Useful for monitoring and testing.
    #[must_use]
    pub fn dirty_count(&self) -> usize {
        self.dirty_clusters.read().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = ClusterDetectionConfig::default();
        assert_eq!(config.debounce_ms, 100);
        assert_eq!(config.min_gap, Decimal::new(2, 2));
        assert_eq!(config.max_clusters_per_cycle, 50);
    }

    #[test]
    fn test_handle_creation() {
        let (tx, _rx) = mpsc::channel(1);
        let handle = ClusterDetectionHandle { shutdown_tx: tx };
        // Handle should be created successfully
        drop(handle);
    }
}
