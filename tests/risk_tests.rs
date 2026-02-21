//! Tests for risk management with concurrent exposure checks.

use std::sync::Arc;

use edgelord::adapters::risk::RiskManager;
use edgelord::domain::{MarketId, Opportunity, OpportunityLeg, TokenId};
use edgelord::runtime::{AppState, RiskLimits};
use rust_decimal_macros::dec;
use tokio::sync::Barrier;

/// Test that concurrent opportunities cannot exceed total exposure limit.
///
/// This test verifies that when multiple opportunities are checked concurrently,
/// the system atomically reserves exposure so that the total (current + pending)
/// never exceeds the limit.
#[tokio::test]
async fn concurrent_opportunities_cannot_exceed_total_exposure() {
    // Set up: limit of $100, each opportunity costs $60
    // If 2 opportunities are checked concurrently, only 1 should be approved
    let limits = RiskLimits {
        max_total_exposure: dec!(100),
        min_profit_threshold: dec!(0),
        ..Default::default()
    };
    let state = Arc::new(AppState::new(limits));

    // Create 2 opportunities, each requiring $60 exposure
    // Together they would be $120, exceeding the $100 limit
    let opp1 = Opportunity::new(
        MarketId::from("market-1"),
        "Test 1?",
        vec![
            OpportunityLeg::new(TokenId::from("yes-1"), dec!(0.30)),
            OpportunityLeg::new(TokenId::from("no-1"), dec!(0.30)),
        ],
        dec!(100), // volume
        dec!(1.0), // payout
    );
    // total_cost = 0.30 + 0.30 = 0.60, exposure = 0.60 * 100 = $60

    let opp2 = Opportunity::new(
        MarketId::from("market-2"),
        "Test 2?",
        vec![
            OpportunityLeg::new(TokenId::from("yes-2"), dec!(0.30)),
            OpportunityLeg::new(TokenId::from("no-2"), dec!(0.30)),
        ],
        dec!(100), // volume
        dec!(1.0), // payout
    );
    // total_cost = 0.30 + 0.30 = 0.60, exposure = 0.60 * 100 = $60

    // Use a barrier to synchronize concurrent checks
    let barrier = Arc::new(Barrier::new(2));
    let barrier1 = barrier.clone();
    let barrier2 = barrier.clone();

    let state1 = state.clone();
    let state2 = state.clone();

    // Spawn two concurrent checks
    let handle1 = tokio::spawn(async move {
        barrier1.wait().await;
        let risk1 = RiskManager::new(state1.clone());
        let result = risk1.check(&opp1);
        (result.is_approved(), state1)
    });

    let handle2 = tokio::spawn(async move {
        barrier2.wait().await;
        let risk2 = RiskManager::new(state2.clone());
        let result = risk2.check(&opp2);
        (result.is_approved(), state2)
    });

    let (approved1, _state1) = handle1.await.unwrap();
    let (approved2, _state2) = handle2.await.unwrap();

    // At most one should be approved
    let approved_count = (if approved1 { 1 } else { 0 }) + (if approved2 { 1 } else { 0 });
    assert!(
        approved_count <= 1,
        "Expected at most 1 approval, got {} (opp1: {}, opp2: {})",
        approved_count,
        approved1,
        approved2
    );

    // Verify total exposure (current + pending) never exceeded limit
    let total_exposure = state.total_exposure();
    let pending_exposure = state.pending_exposure();
    assert!(
        total_exposure + pending_exposure <= dec!(100),
        "Total exposure ({}) + pending ({}) = {} exceeds limit of 100",
        total_exposure,
        pending_exposure,
        total_exposure + pending_exposure
    );
}
