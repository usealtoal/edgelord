//! Integration tests for the cluster detection service.

mod support;

use std::sync::Arc;

use chrono::Duration;
use rust_decimal_macros::dec;

use edgelord::adapters::cluster::{
    ClusterDetectionConfig, ClusterDetectionService, ClusterDetector,
};
use edgelord::domain::{MarketId, MarketRegistry, Relation, RelationKind};
use edgelord::runtime::cache::{ClusterCache, BookCache};

fn setup_test_environment() -> (Arc<BookCache>, Arc<ClusterCache>, Arc<MarketRegistry>) {
    let markets = vec![
        support::market::make_binary_market(
            "market-a",
            "Will market-a happen?",
            "yes-a",
            "no-a",
            dec!(1.00),
        ),
        support::market::make_binary_market(
            "market-b",
            "Will market-b happen?",
            "yes-b",
            "no-b",
            dec!(1.00),
        ),
        support::market::make_binary_market(
            "market-c",
            "Will market-c happen?",
            "yes-c",
            "no-c",
            dec!(1.00),
        ),
    ];

    let registry = support::registry::make_registry(markets);

    let cache = BookCache::new();
    support::book::set_book(&cache, "yes-a", dec!(0.40), dec!(0.42));
    support::book::set_book(&cache, "yes-b", dec!(0.55), dec!(0.57));
    support::book::set_book(&cache, "yes-c", dec!(0.30), dec!(0.32));

    let cluster_cache = ClusterCache::new(Duration::hours(1));

    let relation =
        support::relation::mutually_exclusive(&["market-a", "market-b"], 0.95, "Test relation");
    cluster_cache.put_relations(vec![relation]);

    (Arc::new(cache), Arc::new(cluster_cache), Arc::new(registry))
}

#[test]
fn test_detector_with_valid_cluster() {
    let (cache, cluster_cache, registry) = setup_test_environment();

    let config = ClusterDetectionConfig {
        min_gap: dec!(0.001), // Very low threshold to detect anything
        ..Default::default()
    };
    let detector = ClusterDetector::new(config);

    let clusters = cluster_cache.all_clusters();
    assert!(!clusters.is_empty(), "Should have at least one cluster");

    let cluster = &clusters[0];
    let result = detector.detect(cluster, &cache, &registry);

    // Should succeed (either Some or None depending on gap)
    assert!(
        result.is_ok(),
        "Detection should not error: {:?}",
        result.err()
    );
}

#[test]
fn test_detector_missing_price_data() {
    let markets = vec![
        support::market::make_binary_market(
            "market-x",
            "Will market-x happen?",
            "yes-x",
            "no-x",
            dec!(1.00),
        ),
        support::market::make_binary_market(
            "market-y",
            "Will market-y happen?",
            "yes-y",
            "no-y",
            dec!(1.00),
        ),
    ];
    let registry = Arc::new(support::registry::make_registry(markets));

    // Empty cache - no price data
    let cache = Arc::new(BookCache::new());
    let cluster_cache = Arc::new(ClusterCache::new(Duration::hours(1)));

    let relation = support::relation::mutually_exclusive(&["market-x", "market-y"], 0.95, "Test");
    cluster_cache.put_relations(vec![relation]);

    let detector = ClusterDetector::new(ClusterDetectionConfig::default());
    let cluster = &cluster_cache.all_clusters()[0];

    let result = detector.detect(cluster, &cache, &registry);
    assert!(result.is_err(), "Should fail with missing price data");
}

#[test]
fn test_detector_gap_below_threshold() {
    let (cache, cluster_cache, registry) = setup_test_environment();

    // Very high threshold - nothing should pass
    let config = ClusterDetectionConfig {
        min_gap: dec!(0.99),
        ..Default::default()
    };
    let detector = ClusterDetector::new(config);

    let cluster = &cluster_cache.all_clusters()[0];
    let result = detector.detect(cluster, &cache, &registry);

    assert!(result.is_ok());
    assert!(
        result.unwrap().is_none(),
        "Should return None when gap below threshold"
    );
}

#[test]
fn test_service_creation() {
    let (cache, cluster_cache, registry) = setup_test_environment();
    let config = ClusterDetectionConfig::default();

    let service = ClusterDetectionService::new(config, cache, cluster_cache, registry);
    assert_eq!(service.dirty_count(), 0);
}

#[tokio::test]
async fn test_service_with_notifications() {
    let (order_cache, update_rx) = BookCache::with_notifications(16);
    let order_cache = Arc::new(order_cache);

    let markets = vec![
        support::market::make_binary_market("m1", "Will m1 happen?", "yes-1", "no-1", dec!(1.00)),
        support::market::make_binary_market("m2", "Will m2 happen?", "yes-2", "no-2", dec!(1.00)),
    ];
    let registry = Arc::new(support::registry::make_registry(markets));

    let cluster_cache = Arc::new(ClusterCache::new(Duration::hours(1)));
    let relation = support::relation::mutually_exclusive(&["m1", "m2"], 0.95, "Test");
    cluster_cache.put_relations(vec![relation]);

    let config = ClusterDetectionConfig::default();
    let service =
        ClusterDetectionService::new(config, Arc::clone(&order_cache), cluster_cache, registry);

    let (handle, _opp_rx) = service.start(update_rx);

    // Update order book - should trigger notification
    support::book::set_book(&order_cache, "yes-1", dec!(0.45), dec!(0.47));

    // Give the service time to process
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Shutdown gracefully
    handle.shutdown().await;
}

#[test]
fn test_cluster_with_three_markets() {
    let markets = vec![
        support::market::make_binary_market(
            "trump",
            "Will trump happen?",
            "yes-trump",
            "no-trump",
            dec!(1.00),
        ),
        support::market::make_binary_market(
            "biden",
            "Will biden happen?",
            "yes-biden",
            "no-biden",
            dec!(1.00),
        ),
        support::market::make_binary_market(
            "other",
            "Will other happen?",
            "yes-other",
            "no-other",
            dec!(1.00),
        ),
    ];
    let registry = Arc::new(support::registry::make_registry(markets));

    let cache = BookCache::new();
    support::book::set_book(&cache, "yes-trump", dec!(0.40), dec!(0.42));
    support::book::set_book(&cache, "yes-biden", dec!(0.35), dec!(0.37));
    support::book::set_book(&cache, "yes-other", dec!(0.20), dec!(0.22));
    let cache = Arc::new(cache);

    let cluster_cache = Arc::new(ClusterCache::new(Duration::hours(1)));

    // Exactly one must win (should sum to 1)
    let relation =
        support::relation::exactly_one(&["trump", "biden", "other"], 0.98, "Presidential election");
    cluster_cache.put_relations(vec![relation]);

    let config = ClusterDetectionConfig {
        min_gap: dec!(0.001),
        ..Default::default()
    };
    let detector = ClusterDetector::new(config);

    let cluster = &cluster_cache.all_clusters()[0];
    let result = detector.detect(cluster, &cache, &registry);

    assert!(result.is_ok());
    // With sum = 0.42 + 0.37 + 0.22 = 1.01, there might be a small gap
}

#[test]
fn test_implies_relation() {
    let markets = vec![
        support::market::make_binary_market(
            "pa-win",
            "Will PA win?",
            "yes-pa",
            "no-pa",
            dec!(1.00),
        ),
        support::market::make_binary_market(
            "swing-win",
            "Will swing win?",
            "yes-swing",
            "no-swing",
            dec!(1.00),
        ),
    ];
    let registry = Arc::new(support::registry::make_registry(markets));

    let cache = BookCache::new();
    // PA win at 0.45, swing win at 0.40 - violation! (PA implies swing)
    support::book::set_book(&cache, "yes-pa", dec!(0.43), dec!(0.45));
    support::book::set_book(&cache, "yes-swing", dec!(0.38), dec!(0.40));
    let cache = Arc::new(cache);

    let cluster_cache = Arc::new(ClusterCache::new(Duration::hours(1)));

    let relation = Relation::new(
        RelationKind::Implies {
            if_yes: MarketId::from("pa-win"),
            then_yes: MarketId::from("swing-win"),
        },
        0.95,
        "PA is a swing state",
    );
    cluster_cache.put_relations(vec![relation]);

    let config = ClusterDetectionConfig {
        min_gap: dec!(0.001),
        ..Default::default()
    };
    let detector = ClusterDetector::new(config);

    let cluster = &cluster_cache.all_clusters()[0];
    let result = detector.detect(cluster, &cache, &registry);

    assert!(result.is_ok());
}

#[test]
fn test_empty_cluster_cache() {
    let registry = Arc::new(MarketRegistry::new());
    let cache = Arc::new(BookCache::new());
    let cluster_cache = Arc::new(ClusterCache::new(Duration::hours(1)));

    let config = ClusterDetectionConfig::default();
    let service = ClusterDetectionService::new(config, cache, cluster_cache, registry);

    // Should handle empty state gracefully
    assert_eq!(service.dirty_count(), 0);
}
