//! Inference startup and background wiring.

use std::sync::Arc;

use tracing::info;

use crate::application::cache::cluster::ClusterCache;
use crate::application::inference::service::{
    run_full_inference, InferenceService, InferenceServiceHandle,
};
use crate::infrastructure::config::settings::Config;
use crate::port::outbound::inference::{MarketSummary, RelationInferrer};
use crate::port::outbound::notifier::{Event, NotifierRegistry, RelationDetail, RelationsEvent};

/// Run startup inference pass and notify on discovered relations.
pub(crate) async fn run_startup_inference(
    config: &Config,
    inferrer: Option<&Arc<dyn RelationInferrer>>,
    market_summaries: &[MarketSummary],
    cluster_cache: &ClusterCache,
    notifiers: &Arc<NotifierRegistry>,
) {
    let Some(inferrer) = inferrer else {
        return;
    };

    info!(
        markets = market_summaries.len(),
        batch_size = config.inference.batch_size,
        "Running full startup inference"
    );
    let result = run_full_inference(
        inferrer.as_ref(),
        market_summaries,
        config.inference.batch_size,
        cluster_cache,
    )
    .await;

    info!(
        markets = result.markets_processed,
        relations = result.relations_discovered,
        batches = result.batches_run,
        "Startup inference complete"
    );

    if result.relations.is_empty() {
        return;
    }

    let relation_details: Vec<RelationDetail> = result
        .relations
        .iter()
        .map(|relation| {
            let market_questions: Vec<String> = relation
                .kind
                .market_ids()
                .iter()
                .filter_map(|id| {
                    market_summaries
                        .iter()
                        .find(|summary| &summary.id == *id)
                        .map(|summary| summary.question.clone())
                })
                .collect();

            RelationDetail {
                relation_type: relation.kind.type_name().to_string(),
                confidence: relation.confidence,
                market_questions,
                reasoning: relation.reasoning.clone(),
            }
        })
        .collect();

    notifiers.notify_all(Event::RelationsDiscovered(RelationsEvent {
        relations_count: result.relations_discovered,
        relations: relation_details,
    }));
}

/// Start periodic inference service and attach logging task.
pub(crate) fn start_continuous_inference(
    config: &Config,
    inferrer: Option<Arc<dyn RelationInferrer>>,
    cluster_cache: Arc<ClusterCache>,
    market_summaries: Arc<Vec<MarketSummary>>,
) -> Option<InferenceServiceHandle> {
    if !config.inference.enabled {
        return None;
    }

    let inferrer = inferrer?;

    let service = InferenceService::new(
        inferrer,
        config.inference.clone(),
        Arc::clone(&cluster_cache),
    );
    let (handle, mut result_rx) = service.start(market_summaries);

    tokio::spawn(async move {
        while let Some(result) = result_rx.recv().await {
            info!(
                markets = result.markets_processed,
                relations = result.relations_discovered,
                batches = result.batches_run,
                "Periodic inference complete"
            );
        }
    });

    info!(
        interval_secs = config.inference.scan_interval_seconds,
        "Continuous inference service started"
    );

    Some(handle)
}
