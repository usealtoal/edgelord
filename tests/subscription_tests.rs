//! Integration tests for subscription management.
//!
//! Tests the end-to-end flow of filtering, scoring, and subscription management
//! for adaptive market subscription.

use edgelord::app::{PolymarketFilterConfig, PolymarketScoringConfig};
use edgelord::core::domain::{MarketId, MarketScore, ScoreFactors, TokenId};
use edgelord::core::exchange::polymarket::{PolymarketFilter, PolymarketScorer};
use edgelord::core::exchange::{MarketFilter, MarketInfo, MarketScorer, OutcomeInfo};
use edgelord::core::service::{PrioritySubscriptionManager, SubscriptionManager};

fn make_market(id: &str, outcomes: usize) -> MarketInfo {
    let outcome_infos: Vec<_> = (0..outcomes)
        .map(|i| OutcomeInfo {
            token_id: format!("{id}-token-{i}"),
            name: if i == 0 {
                "Yes".to_string()
            } else {
                "No".to_string()
            },
            price: None,
        })
        .collect();

    MarketInfo {
        id: id.to_string(),
        question: format!("Question for {id}?"),
        outcomes: outcome_infos,
        active: true,
    }
}

#[tokio::test]
async fn test_filter_score_subscribe_flow() {
    // Create components
    let filter_config = PolymarketFilterConfig::default();
    let filter = PolymarketFilter::new(&filter_config);

    let scoring_config = PolymarketScoringConfig::default();
    let scorer = PolymarketScorer::new(&scoring_config);

    let manager = PrioritySubscriptionManager::new(100);

    // Create test markets
    let markets: Vec<_> = (0..10)
        .map(|i| make_market(&format!("market-{i}"), 2))
        .collect();

    // Filter markets
    let eligible = filter.filter(&markets);
    assert_eq!(eligible.len(), 10);

    // Score markets
    let mut scores = Vec::new();
    for market in &eligible {
        let score = scorer.score(market).await.unwrap();

        // Register token mapping
        let tokens: Vec<_> = market
            .outcomes
            .iter()
            .map(|o| TokenId::new(&o.token_id))
            .collect();
        manager.register_market_tokens(MarketId::new(&market.id), tokens);

        scores.push(score);
    }

    // Enqueue and expand
    manager.enqueue(scores);
    assert_eq!(manager.pending_count(), 10);

    let added = manager.expand(5).await.unwrap();
    assert_eq!(added.len(), 10); // 5 markets * 2 tokens
    assert_eq!(manager.active_count(), 10);
    assert_eq!(manager.pending_count(), 5);
}

#[tokio::test]
async fn test_subscription_limits_respected() {
    let manager = PrioritySubscriptionManager::new(4); // Only 4 tokens allowed

    // Register 3 markets with 2 tokens each
    for i in 0..3 {
        manager.register_market_tokens(
            MarketId::new(&format!("m{i}")),
            vec![
                TokenId::new(&format!("m{i}-yes")),
                TokenId::new(&format!("m{i}-no")),
            ],
        );
    }

    let scores: Vec<_> = (0..3)
        .map(|i| {
            MarketScore::new(
                MarketId::new(&format!("m{i}")),
                ScoreFactors::default(),
                1.0 - (i as f64 * 0.1), // Decreasing priority
            )
        })
        .collect();

    manager.enqueue(scores);

    // Try to expand all 3, but limit is 4 tokens
    let added = manager.expand(10).await.unwrap();
    assert_eq!(added.len(), 4); // Only 2 markets fit (4 tokens)
    assert_eq!(manager.active_count(), 4);
}

#[tokio::test]
async fn test_priority_ordering() {
    let manager = PrioritySubscriptionManager::new(100);

    // Register 3 markets with different priorities
    for i in 0..3 {
        manager.register_market_tokens(
            MarketId::new(&format!("priority-{i}")),
            vec![TokenId::new(&format!("priority-{i}-token"))],
        );
    }

    // Enqueue with different scores (higher score = higher priority)
    let scores = vec![
        MarketScore::new(
            MarketId::new("priority-0"),
            ScoreFactors::default(),
            0.3, // Low priority
        ),
        MarketScore::new(
            MarketId::new("priority-1"),
            ScoreFactors::default(),
            0.9, // High priority
        ),
        MarketScore::new(
            MarketId::new("priority-2"),
            ScoreFactors::default(),
            0.6, // Medium priority
        ),
    ];

    manager.enqueue(scores);

    // Expand by 1 - should get highest priority market
    let added = manager.expand(1).await.unwrap();
    assert_eq!(added.len(), 1);
    assert_eq!(added[0].as_str(), "priority-1-token");

    // Expand by 1 more - should get medium priority
    let added = manager.expand(1).await.unwrap();
    assert_eq!(added.len(), 1);
    assert_eq!(added[0].as_str(), "priority-2-token");

    // Expand by 1 more - should get low priority
    let added = manager.expand(1).await.unwrap();
    assert_eq!(added.len(), 1);
    assert_eq!(added[0].as_str(), "priority-0-token");
}

#[tokio::test]
async fn test_filter_excludes_inactive_markets() {
    let filter_config = PolymarketFilterConfig::default();
    let filter = PolymarketFilter::new(&filter_config);

    let markets = vec![
        MarketInfo {
            id: "active-1".to_string(),
            question: "Active market?".to_string(),
            outcomes: vec![
                OutcomeInfo {
                    token_id: "active-yes".to_string(),
                    name: "Yes".to_string(),
                    price: None,
                },
                OutcomeInfo {
                    token_id: "active-no".to_string(),
                    name: "No".to_string(),
                    price: None,
                },
            ],
            active: true,
        },
        MarketInfo {
            id: "inactive-1".to_string(),
            question: "Inactive market?".to_string(),
            outcomes: vec![
                OutcomeInfo {
                    token_id: "inactive-yes".to_string(),
                    name: "Yes".to_string(),
                    price: None,
                },
                OutcomeInfo {
                    token_id: "inactive-no".to_string(),
                    name: "No".to_string(),
                    price: None,
                },
            ],
            active: false,
        },
    ];

    let eligible = filter.filter(&markets);

    assert_eq!(eligible.len(), 1);
    assert_eq!(eligible[0].id, "active-1");
}

#[tokio::test]
async fn test_scorer_produces_valid_scores() {
    let scoring_config = PolymarketScoringConfig::default();
    let scorer = PolymarketScorer::new(&scoring_config);

    let binary_market = make_market("binary", 2);
    let multi_market = make_market("multi", 6);

    let binary_score = scorer.score(&binary_market).await.unwrap();
    let multi_score = scorer.score(&multi_market).await.unwrap();

    // Scores should be valid (between 0 and 1)
    assert!(binary_score.composite() >= 0.0 && binary_score.composite() <= 1.0);
    assert!(multi_score.composite() >= 0.0 && multi_score.composite() <= 1.0);

    // Market IDs should be correct
    assert_eq!(binary_score.market_id().as_str(), "binary");
    assert_eq!(multi_score.market_id().as_str(), "multi");
}

#[tokio::test]
async fn test_contract_removes_subscriptions() {
    let manager = PrioritySubscriptionManager::new(100);

    // Register and subscribe to markets
    for i in 0..3 {
        manager.register_market_tokens(
            MarketId::new(&format!("market-{i}")),
            vec![TokenId::new(&format!("market-{i}-token"))],
        );
    }

    let scores: Vec<_> = (0..3)
        .map(|i| {
            MarketScore::new(
                MarketId::new(&format!("market-{i}")),
                ScoreFactors::default(),
                0.5,
            )
        })
        .collect();

    manager.enqueue(scores);
    manager.expand(3).await.unwrap();

    assert_eq!(manager.active_count(), 3);

    // Contract by 2
    let removed = manager.contract(2).await.unwrap();

    assert_eq!(removed.len(), 2);
    assert_eq!(manager.active_count(), 1);
}
