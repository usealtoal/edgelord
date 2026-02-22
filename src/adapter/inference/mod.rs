//! Inference service for continuous market relation discovery.
//!
//! Handles both multi-batch startup inference and periodic cycling
//! to maximize market coverage for arbitrage detection.

mod inferrer;

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::domain::Relation;
use crate::port::{MarketSummary, RelationInferrer};
use crate::runtime::cache::ClusterCache;
use crate::runtime::InferenceConfig;

// Re-export LlmInferrer
pub use inferrer::LlmInferrer;

// Type alias for backward compatibility
pub type Inferrer = dyn RelationInferrer;

/// Handle to control the inference service.
pub struct InferenceServiceHandle {
    shutdown_tx: mpsc::Sender<()>,
}

impl InferenceServiceHandle {
    /// Signal the service to shut down gracefully.
    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(()).await;
    }
}

/// Result of running inference on all markets.
#[derive(Debug, Clone)]
pub struct InferenceResult {
    /// Total markets processed.
    pub markets_processed: usize,
    /// Total relations discovered.
    pub relations_discovered: usize,
    /// Number of batches run.
    pub batches_run: usize,
    /// All discovered relations (for notifications).
    pub relations: Vec<Relation>,
}

/// Run inference on all markets in batches.
///
/// This processes ALL markets by chunking them into batches and running
/// inference on each batch sequentially.
pub async fn run_full_inference(
    inferrer: &dyn RelationInferrer,
    markets: &[MarketSummary],
    batch_size: usize,
    cluster_cache: &ClusterCache,
) -> InferenceResult {
    let mut total_relations = 0;
    let mut batches_run = 0;
    let mut all_relations = Vec::new();

    if markets.len() < 2 {
        debug!("Not enough markets for inference");
        return InferenceResult {
            markets_processed: markets.len(),
            relations_discovered: 0,
            batches_run: 0,
            relations: vec![],
        };
    }

    for (batch_idx, chunk) in markets.chunks(batch_size).enumerate() {
        if chunk.len() < 2 {
            debug!(
                batch = batch_idx,
                markets = chunk.len(),
                "Skipping small batch"
            );
            continue;
        }

        debug!(
            batch = batch_idx,
            markets = chunk.len(),
            "Running inference batch"
        );

        match inferrer.infer(chunk).await {
            Ok(relations) => {
                if !relations.is_empty() {
                    info!(
                        batch = batch_idx,
                        relations = relations.len(),
                        "Discovered relations"
                    );
                    total_relations += relations.len();
                    all_relations.extend(relations.clone());
                    cluster_cache.put_relations(relations);
                }
                batches_run += 1;
            }
            Err(e) => {
                warn!(batch = batch_idx, error = %e, "Inference batch failed");
            }
        }
    }

    info!(
        markets = markets.len(),
        relations = total_relations,
        batches = batches_run,
        "Full inference complete"
    );

    InferenceResult {
        markets_processed: markets.len(),
        relations_discovered: total_relations,
        batches_run,
        relations: all_relations,
    }
}

/// Inference service for continuous relation discovery.
pub struct InferenceService {
    inferrer: Arc<dyn RelationInferrer>,
    config: InferenceConfig,
    cluster_cache: Arc<ClusterCache>,
}

impl InferenceService {
    /// Create a new inference service.
    pub fn new(
        inferrer: Arc<dyn RelationInferrer>,
        config: InferenceConfig,
        cluster_cache: Arc<ClusterCache>,
    ) -> Self {
        Self {
            inferrer,
            config,
            cluster_cache,
        }
    }

    /// Start the continuous inference service.
    ///
    /// Returns a handle to control the service and a receiver for results.
    pub fn start(
        self,
        markets: Arc<Vec<MarketSummary>>,
    ) -> (InferenceServiceHandle, mpsc::Receiver<InferenceResult>) {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        let (result_tx, result_rx) = mpsc::channel::<InferenceResult>(16);

        let interval = Duration::from_secs(self.config.scan_interval_seconds);
        let batch_size = self.config.batch_size;
        let inferrer = self.inferrer;
        let cluster_cache = self.cluster_cache;

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            // Skip the first immediate tick - startup inference is handled separately
            interval_timer.tick().await;

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        info!("Inference service shutting down");
                        break;
                    }
                    _ = interval_timer.tick() => {
                        info!(markets = markets.len(), "Running periodic inference");
                        let result = run_full_inference(
                            inferrer.as_ref(),
                            &markets,
                            batch_size,
                            &cluster_cache,
                        ).await;

                        if result_tx.send(result).await.is_err() {
                            debug!("Inference result receiver dropped");
                            break;
                        }
                    }
                }
            }
        });

        (InferenceServiceHandle { shutdown_tx }, result_rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{MarketId, Relation, RelationKind};
    use crate::port::tests::MockInferrer;
    use chrono::Duration as ChronoDuration;

    fn sample_markets(count: usize) -> Vec<MarketSummary> {
        (0..count)
            .map(|i| MarketSummary {
                id: MarketId::new(format!("market-{}", i)),
                question: format!("Will event {} happen?", i),
                outcomes: vec!["Yes".into(), "No".into()],
            })
            .collect()
    }

    fn sample_relation() -> Relation {
        Relation::new(
            RelationKind::MutuallyExclusive {
                markets: vec![MarketId::new("market-0"), MarketId::new("market-1")],
            },
            0.95,
            "Test relation",
        )
    }

    #[tokio::test]
    async fn run_full_inference_processes_all_batches() {
        let markets = sample_markets(250);
        let inferrer = MockInferrer::new(vec![sample_relation()]);
        let cache = ClusterCache::new(ChronoDuration::hours(1));

        let result = run_full_inference(&inferrer, &markets, 100, &cache).await;

        // 250 markets / 100 batch = 3 batches
        assert_eq!(result.batches_run, 3);
        assert_eq!(result.markets_processed, 250);
        // Each batch returns 1 relation
        assert_eq!(result.relations_discovered, 3);
    }

    #[tokio::test]
    async fn run_full_inference_skips_small_batches() {
        let markets = sample_markets(1); // Only 1 market
        let inferrer = MockInferrer::new(vec![sample_relation()]);
        let cache = ClusterCache::new(ChronoDuration::hours(1));

        let result = run_full_inference(&inferrer, &markets, 100, &cache).await;

        assert_eq!(result.batches_run, 0);
        assert_eq!(result.relations_discovered, 0);
    }

    #[tokio::test]
    async fn run_full_inference_stores_relations_in_cache() {
        let markets = sample_markets(50);
        let inferrer = MockInferrer::new(vec![sample_relation()]);
        let cache = ClusterCache::new(ChronoDuration::hours(1));

        run_full_inference(&inferrer, &markets, 100, &cache).await;

        // Verify relations were stored
        assert!(cache.has_relations(&MarketId::new("market-0")));
        assert!(cache.has_relations(&MarketId::new("market-1")));
    }

    #[tokio::test]
    async fn run_full_inference_handles_empty_relations() {
        let markets = sample_markets(50);
        let inferrer = MockInferrer::new(vec![]); // No relations
        let cache = ClusterCache::new(ChronoDuration::hours(1));

        let result = run_full_inference(&inferrer, &markets, 100, &cache).await;

        assert_eq!(result.batches_run, 1);
        assert_eq!(result.relations_discovered, 0);
    }

    #[tokio::test]
    async fn inference_service_can_be_shutdown() {
        let inferrer = Arc::new(MockInferrer::new(vec![]));
        let cache = Arc::new(ClusterCache::new(ChronoDuration::hours(1)));
        let config = InferenceConfig {
            scan_interval_seconds: 1, // 1 second for test
            batch_size: 10,
            ..Default::default()
        };

        let service = InferenceService::new(inferrer, config, cache);
        let markets = Arc::new(sample_markets(20));

        let (handle, _rx) = service.start(markets);

        // Should not hang
        handle.shutdown().await;
    }
}
