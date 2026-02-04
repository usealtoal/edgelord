//! Integration tests for the cluster detection service.

use std::sync::Arc;

use chrono::Duration;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use edgelord::core::cache::{ClusterCache, OrderBookCache};
use edgelord::core::domain::{
    Market, MarketId, MarketRegistry, OrderBook, Outcome, PriceLevel, Relation,
    RelationKind, TokenId,
};
use edgelord::core::service::cluster::{ClusterDetectionConfig, ClusterDetectionService, ClusterDetector};

fn create_test_market(id: &str, yes_token: &str, no_token: &str) -> Market {
    let outcomes = vec![
        Outcome::new(TokenId::from(yes_token), "Yes"),
        Outcome::new(TokenId::from(no_token), "No"),
    ];
    Market::new(
        MarketId::from(id),
        format!("Will {id} happen?"),
        outcomes,
        dec!(1.00),
    )
}

fn create_order_book(token_id: &str, bid: Decimal, ask: Decimal) -> OrderBook {
    OrderBook::with_levels(
        TokenId::from(token_id),
        vec![PriceLevel::new(bid, dec!(100))],
        vec![PriceLevel::new(ask, dec!(100))],
    )
}

fn setup_test_environment() -> (Arc<OrderBookCache>, Arc<ClusterCache>, Arc<MarketRegistry>) {
    let mut registry = MarketRegistry::new();
    registry.add(create_test_market("market-a", "yes-a", "no-a"));
    registry.add(create_test_market("market-b", "yes-b", "no-b"));
    registry.add(create_test_market("market-c", "yes-c", "no-c"));

    let cache = OrderBookCache::new();
    cache.update(create_order_book("yes-a", dec!(0.40), dec!(0.42)));
    cache.update(create_order_book("yes-b", dec!(0.55), dec!(0.57)));
    cache.update(create_order_book("yes-c", dec!(0.30), dec!(0.32)));

    let cluster_cache = ClusterCache::new(Duration::hours(1));

    // Create a mutual exclusion relation between market-a and market-b
    let relation = Relation::new(
        RelationKind::MutuallyExclusive {
            markets: vec![MarketId::from("market-a"), MarketId::from("market-b")],
        },
        0.95,
        "Test relation",
    );
    cluster_cache.put_relations(vec![relation]);

    (
        Arc::new(cache),
        Arc::new(cluster_cache),
        Arc::new(registry),
    )
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
    assert!(result.is_ok(), "Detection should not error: {:?}", result.err());
}

#[test]
fn test_detector_missing_price_data() {
    let mut registry = MarketRegistry::new();
    registry.add(create_test_market("market-x", "yes-x", "no-x"));
    registry.add(create_test_market("market-y", "yes-y", "no-y"));
    let registry = Arc::new(registry);

    // Empty cache - no price data
    let cache = Arc::new(OrderBookCache::new());
    let cluster_cache = Arc::new(ClusterCache::new(Duration::hours(1)));

    let relation = Relation::new(
        RelationKind::MutuallyExclusive {
            markets: vec![MarketId::from("market-x"), MarketId::from("market-y")],
        },
        0.95,
        "Test",
    );
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
    assert!(result.unwrap().is_none(), "Should return None when gap below threshold");
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
    let (order_cache, update_rx) = OrderBookCache::with_notifications(16);
    let order_cache = Arc::new(order_cache);

    let mut registry = MarketRegistry::new();
    registry.add(create_test_market("m1", "yes-1", "no-1"));
    registry.add(create_test_market("m2", "yes-2", "no-2"));
    let registry = Arc::new(registry);

    let cluster_cache = Arc::new(ClusterCache::new(Duration::hours(1)));
    let relation = Relation::new(
        RelationKind::MutuallyExclusive {
            markets: vec![MarketId::from("m1"), MarketId::from("m2")],
        },
        0.95,
        "Test",
    );
    cluster_cache.put_relations(vec![relation]);

    let config = ClusterDetectionConfig::default();
    let service = ClusterDetectionService::new(
        config,
        Arc::clone(&order_cache),
        cluster_cache,
        registry,
    );

    let (handle, _opp_rx) = service.start(update_rx);

    // Update order book - should trigger notification
    order_cache.update(create_order_book("yes-1", dec!(0.45), dec!(0.47)));

    // Give the service time to process
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Shutdown gracefully
    handle.shutdown().await;
}

#[test]
fn test_cluster_with_three_markets() {
    let mut registry = MarketRegistry::new();
    registry.add(create_test_market("trump", "yes-trump", "no-trump"));
    registry.add(create_test_market("biden", "yes-biden", "no-biden"));
    registry.add(create_test_market("other", "yes-other", "no-other"));
    let registry = Arc::new(registry);

    let cache = OrderBookCache::new();
    cache.update(create_order_book("yes-trump", dec!(0.40), dec!(0.42)));
    cache.update(create_order_book("yes-biden", dec!(0.35), dec!(0.37)));
    cache.update(create_order_book("yes-other", dec!(0.20), dec!(0.22)));
    let cache = Arc::new(cache);

    let cluster_cache = Arc::new(ClusterCache::new(Duration::hours(1)));

    // Exactly one must win (should sum to 1)
    let relation = Relation::new(
        RelationKind::ExactlyOne {
            markets: vec![
                MarketId::from("trump"),
                MarketId::from("biden"),
                MarketId::from("other"),
            ],
        },
        0.98,
        "Presidential election",
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
    // With sum = 0.42 + 0.37 + 0.22 = 1.01, there might be a small gap
}

#[test]
fn test_implies_relation() {
    let mut registry = MarketRegistry::new();
    registry.add(create_test_market("pa-win", "yes-pa", "no-pa"));
    registry.add(create_test_market("swing-win", "yes-swing", "no-swing"));
    let registry = Arc::new(registry);

    let cache = OrderBookCache::new();
    // PA win at 0.45, swing win at 0.40 - violation! (PA implies swing)
    cache.update(create_order_book("yes-pa", dec!(0.43), dec!(0.45)));
    cache.update(create_order_book("yes-swing", dec!(0.38), dec!(0.40)));
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
    let cache = Arc::new(OrderBookCache::new());
    let cluster_cache = Arc::new(ClusterCache::new(Duration::hours(1)));

    let config = ClusterDetectionConfig::default();
    let service = ClusterDetectionService::new(config, cache, cluster_cache, registry);

    // Should handle empty state gracefully
    assert_eq!(service.dirty_count(), 0);
}
