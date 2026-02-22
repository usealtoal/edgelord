use super::*;
use crate::domain::score::ScoreFactors;
use crate::error::Error;

fn make_market_id(name: &str) -> MarketId {
    MarketId::new(name)
}

fn make_token_id(name: &str) -> TokenId {
    TokenId::new(name)
}

fn make_market_score(market_id: &str, composite: f64) -> MarketScore {
    let factors = ScoreFactors::default();
    MarketScore::new(make_market_id(market_id), factors, composite)
}

// --- Constructor tests ---

#[test]
fn new_creates_empty_manager() {
    let manager = PrioritySubscriptionManager::new(100);

    assert_eq!(manager.max_subscriptions(), 100);
    assert_eq!(manager.active_count(), 0);
    assert_eq!(manager.pending_count(), 0);
    assert!(manager.active_subscriptions().is_empty());
}

#[test]
fn new_with_zero_max_subscriptions() {
    let manager = PrioritySubscriptionManager::new(0);
    assert_eq!(manager.max_subscriptions(), 0);
}

// --- register_market_tokens tests ---

#[test]
fn register_market_tokens_stores_mapping() {
    let manager = PrioritySubscriptionManager::new(100);
    let market_id = make_market_id("market-1");
    let tokens = vec![make_token_id("token-1a"), make_token_id("token-1b")];

    manager.register_market_tokens(market_id.clone(), tokens.clone());

    let retrieved = manager.get_market_tokens(&market_id);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().len(), 2);
}

#[test]
fn register_market_tokens_overwrites_existing() {
    let manager = PrioritySubscriptionManager::new(100);
    let market_id = make_market_id("market-1");

    manager.register_market_tokens(market_id.clone(), vec![make_token_id("old-token")]);
    manager.register_market_tokens(
        market_id.clone(),
        vec![make_token_id("new-token-1"), make_token_id("new-token-2")],
    );

    let retrieved = manager.get_market_tokens(&market_id).unwrap();
    assert_eq!(retrieved.len(), 2);
    assert_eq!(retrieved[0].as_str(), "new-token-1");
}

// --- enqueue tests ---

#[test]
fn enqueue_adds_to_pending() {
    let manager = PrioritySubscriptionManager::new(100);
    let score = make_market_score("market-1", 0.5);

    manager.enqueue(vec![score]);

    assert_eq!(manager.pending_count(), 1);
}

#[test]
fn enqueue_multiple_markets() {
    let manager = PrioritySubscriptionManager::new(100);
    let scores = vec![
        make_market_score("market-1", 0.5),
        make_market_score("market-2", 0.7),
        make_market_score("market-3", 0.3),
    ];

    manager.enqueue(scores);

    assert_eq!(manager.pending_count(), 3);
}

#[test]
fn enqueue_skips_already_subscribed() {
    let manager = PrioritySubscriptionManager::new(100);
    let market_id = make_market_id("market-1");
    let tokens = vec![make_token_id("token-1")];

    // Register and subscribe.
    manager.register_market_tokens(market_id.clone(), tokens);

    // Manually add to active markets.
    {
        let mut active = manager.active_markets.write().unwrap();
        active.insert(market_id.clone());
    }

    // Try to enqueue the same market.
    let score = make_market_score("market-1", 0.5);
    manager.enqueue(vec![score]);

    // Should not be added to pending.
    assert_eq!(manager.pending_count(), 0);
}

// --- expand tests ---

#[tokio::test]
async fn expand_adds_highest_priority_first() {
    let manager = PrioritySubscriptionManager::new(100);

    // Register markets with tokens.
    manager.register_market_tokens(make_market_id("low"), vec![make_token_id("low-token")]);
    manager.register_market_tokens(make_market_id("high"), vec![make_token_id("high-token")]);
    manager.register_market_tokens(
        make_market_id("medium"),
        vec![make_token_id("medium-token")],
    );

    // Enqueue in random order.
    manager.enqueue(vec![
        make_market_score("low", 0.2),
        make_market_score("high", 0.9),
        make_market_score("medium", 0.5),
    ]);

    // Expand by 1 - should get the highest priority market.
    let added = manager.expand(1).await.unwrap();

    assert_eq!(added.len(), 1);
    assert_eq!(added[0].as_str(), "high-token");
    assert!(manager.is_subscribed(&make_market_id("high")));
    assert!(!manager.is_subscribed(&make_market_id("medium")));
    assert!(!manager.is_subscribed(&make_market_id("low")));
}

#[tokio::test]
async fn expand_multiple_markets() {
    let manager = PrioritySubscriptionManager::new(100);

    manager.register_market_tokens(make_market_id("market-1"), vec![make_token_id("token-1")]);
    manager.register_market_tokens(make_market_id("market-2"), vec![make_token_id("token-2")]);

    manager.enqueue(vec![
        make_market_score("market-1", 0.5),
        make_market_score("market-2", 0.6),
    ]);

    let added = manager.expand(2).await.unwrap();

    assert_eq!(added.len(), 2);
    assert_eq!(manager.active_count(), 2);
    assert_eq!(manager.pending_count(), 0);
}

#[tokio::test]
async fn expand_respects_max_subscriptions() {
    let manager = PrioritySubscriptionManager::new(2); // Only 2 tokens allowed.

    manager.register_market_tokens(
        make_market_id("market-1"),
        vec![make_token_id("token-1a"), make_token_id("token-1b")], // 2 tokens
    );
    manager.register_market_tokens(make_market_id("market-2"), vec![make_token_id("token-2")]);

    manager.enqueue(vec![
        make_market_score("market-1", 0.5),
        make_market_score("market-2", 0.3),
    ]);

    let added = manager.expand(10).await.unwrap();

    // Should only add market-1's tokens (2 tokens = max).
    assert_eq!(added.len(), 2);
    assert_eq!(manager.active_count(), 2);
    assert!(manager.is_subscribed(&make_market_id("market-1")));
    // market-2 should still be pending.
    assert_eq!(manager.pending_count(), 1);
}

#[tokio::test]
async fn expand_skips_markets_without_token_mapping() {
    let manager = PrioritySubscriptionManager::new(100);

    // Don't register tokens for market-1.
    manager.register_market_tokens(make_market_id("market-2"), vec![make_token_id("token-2")]);

    manager.enqueue(vec![
        make_market_score("market-1", 0.9), // No token mapping
        make_market_score("market-2", 0.5),
    ]);

    let added = manager.expand(2).await.unwrap();

    // Should skip market-1 and add market-2.
    assert_eq!(added.len(), 1);
    assert_eq!(added[0].as_str(), "token-2");
}

#[tokio::test]
async fn expand_returns_empty_when_queue_empty() {
    let manager = PrioritySubscriptionManager::new(100);

    let added = manager.expand(5).await.unwrap();

    assert!(added.is_empty());
}

// --- contract tests ---

#[tokio::test]
async fn contract_removes_tokens_lifo() {
    let manager = PrioritySubscriptionManager::new(100);

    manager.register_market_tokens(make_market_id("market-1"), vec![make_token_id("token-1")]);
    manager.register_market_tokens(make_market_id("market-2"), vec![make_token_id("token-2")]);

    manager.enqueue(vec![
        make_market_score("market-1", 0.5),
        make_market_score("market-2", 0.6),
    ]);

    manager.expand(2).await.unwrap();

    // Contract by 1 - should remove the last added token.
    let removed = manager.contract(1).await.unwrap();

    assert_eq!(removed.len(), 1);
    assert_eq!(manager.active_count(), 1);
}

#[tokio::test]
async fn contract_removes_market_when_no_tokens_left() {
    let manager = PrioritySubscriptionManager::new(100);

    manager.register_market_tokens(make_market_id("market-1"), vec![make_token_id("token-1")]);

    manager.enqueue(vec![make_market_score("market-1", 0.5)]);
    manager.expand(1).await.unwrap();

    assert!(manager.is_subscribed(&make_market_id("market-1")));

    manager.contract(1).await.unwrap();

    assert!(!manager.is_subscribed(&make_market_id("market-1")));
    assert_eq!(manager.active_count(), 0);
}

#[tokio::test]
async fn contract_handles_more_than_active() {
    let manager = PrioritySubscriptionManager::new(100);

    manager.register_market_tokens(make_market_id("market-1"), vec![make_token_id("token-1")]);

    manager.enqueue(vec![make_market_score("market-1", 0.5)]);
    manager.expand(1).await.unwrap();

    // Try to contract more than we have.
    let removed = manager.contract(10).await.unwrap();

    assert_eq!(removed.len(), 1);
    assert_eq!(manager.active_count(), 0);
}

#[tokio::test]
async fn contract_returns_empty_when_no_active() {
    let manager = PrioritySubscriptionManager::new(100);

    let removed = manager.contract(5).await.unwrap();

    assert!(removed.is_empty());
}

// --- is_subscribed tests ---

#[test]
fn is_subscribed_returns_false_for_unknown_market() {
    let manager = PrioritySubscriptionManager::new(100);

    assert!(!manager.is_subscribed(&make_market_id("unknown")));
}

#[tokio::test]
async fn is_subscribed_returns_true_after_expand() {
    let manager = PrioritySubscriptionManager::new(100);

    manager.register_market_tokens(make_market_id("market-1"), vec![make_token_id("token-1")]);
    manager.enqueue(vec![make_market_score("market-1", 0.5)]);

    assert!(!manager.is_subscribed(&make_market_id("market-1")));

    manager.expand(1).await.unwrap();

    assert!(manager.is_subscribed(&make_market_id("market-1")));
}

// --- active_subscriptions tests ---

#[tokio::test]
async fn active_subscriptions_returns_all_tokens() {
    let manager = PrioritySubscriptionManager::new(100);

    manager.register_market_tokens(
        make_market_id("market-1"),
        vec![make_token_id("token-1a"), make_token_id("token-1b")],
    );

    manager.enqueue(vec![make_market_score("market-1", 0.5)]);
    manager.expand(1).await.unwrap();

    let active = manager.active_subscriptions();
    assert_eq!(active.len(), 2);
}

// --- on_connection_event tests ---

#[tokio::test]
async fn on_connection_event_connected() {
    let manager = PrioritySubscriptionManager::new(100);
    let event = ConnectionEvent::Connected { connection_id: 1 };

    let result = manager.on_connection_event(event).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn on_connection_event_disconnected() {
    let manager = PrioritySubscriptionManager::new(100);
    let event = ConnectionEvent::Disconnected {
        connection_id: 1,
        reason: "test".to_string(),
    };

    let result = manager.on_connection_event(event).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn on_connection_event_shard_unhealthy() {
    let manager = PrioritySubscriptionManager::new(100);
    let event = ConnectionEvent::ShardUnhealthy { shard_id: 1 };

    let result = manager.on_connection_event(event).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn on_connection_event_shard_recovered() {
    let manager = PrioritySubscriptionManager::new(100);
    let event = ConnectionEvent::ShardRecovered { shard_id: 1 };

    let result = manager.on_connection_event(event).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn lock_poisoning_does_not_panic() {
    use std::sync::Arc;

    let manager = Arc::new(PrioritySubscriptionManager::new(10));
    let manager_clone = Arc::clone(&manager);

    let handle = std::thread::spawn(move || {
        let _guard = manager_clone.pending.write().unwrap();
        panic!("poison lock");
    });

    assert!(handle.join().is_err());

    let result = manager.expand(1).await;
    assert!(
        matches!(result, Err(Error::Connection(_))),
        "Expected poisoned lock to return a connection error"
    );
}

// --- Thread safety test ---

#[tokio::test]
async fn concurrent_operations() {
    use std::sync::Arc;

    let manager = Arc::new(PrioritySubscriptionManager::new(1000));

    // Register some markets.
    for i in 0..10 {
        manager.register_market_tokens(
            make_market_id(&format!("market-{i}")),
            vec![make_token_id(&format!("token-{i}"))],
        );
    }

    // Spawn concurrent enqueue tasks.
    let mut handles = vec![];
    for i in 0..10 {
        let manager_clone = Arc::clone(&manager);
        handles.push(tokio::spawn(async move {
            let score = make_market_score(&format!("market-{i}"), i as f64 / 10.0);
            manager_clone.enqueue(vec![score]);
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    // All should be in pending queue.
    assert_eq!(manager.pending_count(), 10);

    // Expand all.
    manager.expand(10).await.unwrap();
    assert_eq!(manager.active_count(), 10);
    assert_eq!(manager.pending_count(), 0);
}
