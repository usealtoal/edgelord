//! Integration tests for execution system.

use std::sync::Arc;

use async_trait::async_trait;
use edgelord::app::{spawn_execution, AppState};
use edgelord::core::domain::{
    ArbitrageExecutionResult, FailedLeg, FilledLeg, MarketId, Opportunity, OpportunityLeg,
    OrderId, PositionStatus, TokenId,
};
use edgelord::core::exchange::ArbitrageExecutor;
use edgelord::core::service::{NotifierRegistry, stats};
use edgelord::error::Error;
use rust_decimal_macros::dec;
use tokio::time::{sleep, Duration};

/// Mock executor that returns PartialFill and fails cancel on one leg.
struct MockPartialFillExecutor {
    /// Order IDs that should fail cancellation.
    cancel_fail_order_ids: Vec<String>,
}

#[async_trait]
impl ArbitrageExecutor for MockPartialFillExecutor {
    async fn execute_arbitrage(
        &self,
        _opportunity: &Opportunity,
    ) -> Result<ArbitrageExecutionResult, Error> {
        // Return PartialFill with 2 filled legs and 1 failed leg
        Ok(ArbitrageExecutionResult::PartialFill {
            filled: vec![
                FilledLeg::new(TokenId::from("token-1"), "order-1"),
                FilledLeg::new(TokenId::from("token-2"), "order-2"),
            ],
            failed: vec![FailedLeg::new(
                TokenId::from("token-3"),
                "execution failed",
            )],
        })
    }

    async fn cancel(&self, order_id: &OrderId) -> Result<(), Error> {
        if self.cancel_fail_order_ids.contains(&order_id.as_str().to_string()) {
            Err(Error::Execution(edgelord::error::ExecutionError::OrderRejected(
                "cancel failed".to_string(),
            )))
        } else {
            Ok(())
        }
    }

    fn exchange_name(&self) -> &'static str {
        "mock"
    }
}

#[tokio::test]
async fn partial_fill_triggers_cancel_and_records_partial_position_on_failure() {
    // Arrange: executor returns PartialFill, cancel fails on one leg
    let executor = Arc::new(MockPartialFillExecutor {
        cancel_fail_order_ids: vec!["order-1".to_string()],
    });

    let opportunity = Opportunity::with_strategy(
        MarketId::from("test-market"),
        "Test question?",
        vec![
            OpportunityLeg::new(TokenId::from("token-1"), dec!(0.40)),
            OpportunityLeg::new(TokenId::from("token-2"), dec!(0.50)),
            OpportunityLeg::new(TokenId::from("token-3"), dec!(0.10)),
        ],
        dec!(100),
        dec!(1.00),
        "test-strategy",
    );

    let state = Arc::new(AppState::default());
    let notifiers = Arc::new(NotifierRegistry::new());
    let db_pool = edgelord::core::db::create_pool("sqlite://:memory:").unwrap();
    let stats = stats::create_recorder(db_pool);

    // Execute
    spawn_execution(
        executor,
        opportunity.clone(),
        notifiers,
        state.clone(),
        stats,
        None,
    );

    // Wait for async execution to complete
    sleep(Duration::from_millis(100)).await;

    // Assert: position recorded with PartialFill status and missing leg ids
    let positions = state.positions();
    let all_positions: Vec<_> = positions.all().collect();
    assert_eq!(all_positions.len(), 1, "Expected exactly one position to be recorded");

    let position = &all_positions[0];
    assert_eq!(
        position.market_id().as_str(),
        "test-market",
        "Position should have correct market ID"
    );

    match position.status() {
        PositionStatus::PartialFill { filled, missing } => {
            // Should have 2 filled legs (token-1 and token-2)
            assert_eq!(filled.len(), 2, "Should have 2 filled legs");
            assert_eq!(missing.len(), 1, "Should have 1 missing leg");
            
            // Verify filled token IDs
            let filled_ids: Vec<&str> = filled.iter().map(|t| t.as_str()).collect();
            assert!(filled_ids.contains(&"token-1"), "token-1 should be in filled");
            assert!(filled_ids.contains(&"token-2"), "token-2 should be in filled");
            
            // Verify missing token ID
            let missing_ids: Vec<&str> = missing.iter().map(|t| t.as_str()).collect();
            assert!(missing_ids.contains(&"token-3"), "token-3 should be in missing");
        }
        _ => panic!("Position should have PartialFill status, got {:?}", position.status()),
    }
}
