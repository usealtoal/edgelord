# Code Review Fixes Implementation Plan

> Status: Historical
> Superseded by: N/A
> Summary:
> - Goal: Address all code review findings: execution robustness (partial fills, locking, cancellation), type unification, slippage protection, and dead code cleanup.
> - Scope: Task 1.1: Implement Order Cancellation
> Planned Outcomes:
> - Task 1.1: Implement Order Cancellation
> - Task 1.2: Add Execution Locking


> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Address all code review findings: execution robustness (partial fills, locking, cancellation), type unification, slippage protection, and dead code cleanup.

**Architecture:** Fix execution edge cases first (highest risk), then improve type system (MarketInfo unification), add runtime slippage checks, and finally clean up dead code. Each fix is isolated and testable.

**Tech Stack:** Rust 2021, tokio, rust_decimal, async-trait

---

## Part 1: Execution Robustness (HIGH Priority)

### Task 1.1: Implement Order Cancellation

The `cancel()` method in `OrderExecutor` trait returns "not implemented". We need this for partial fill recovery.

**Files:**
- Modify: `src/adapter/polymarket/executor.rs`

**Step 1: Research Polymarket cancel API**

Check the `polymarket-client-sdk` for cancel order functionality. The SDK's `Client` should have a `cancel_order` method.

**Step 2: Implement cancel method**

```rust
async fn cancel(&self, order_id: &OrderId) -> Result<()> {
    self.client
        .cancel_order(&order_id.as_str())
        .await
        .map_err(|e| ExecutionError::SubmissionFailed(format!("Cancel failed: {e}")))?;

    info!(order_id = %order_id, "Order cancelled");
    Ok(())
}
```

**Step 3: Verify builds**

```bash
cargo build --all-features
```

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: implement order cancellation for Polymarket executor"
```

---

### Task 1.2: Add Execution Locking

Prevent duplicate execution of the same opportunity while one is in-flight.

**Files:**
- Modify: `src/app/state.rs`
- Modify: `src/app/orchestrator.rs`

**Step 1: Add pending executions set to AppState**

In `src/app/state.rs`, add:

```rust
use std::collections::HashSet;
use parking_lot::Mutex;

pub struct AppState {
    positions: RwLock<PositionTracker>,
    risk_limits: RiskLimits,
    pending_executions: Mutex<HashSet<String>>, // market_id -> in-flight
}

impl AppState {
    pub fn new(risk_limits: RiskLimits) -> Self {
        Self {
            positions: RwLock::new(PositionTracker::new()),
            risk_limits,
            pending_executions: Mutex::new(HashSet::new()),
        }
    }

    /// Try to acquire execution lock for a market. Returns false if already locked.
    pub fn try_lock_execution(&self, market_id: &str) -> bool {
        self.pending_executions.lock().insert(market_id.to_string())
    }

    /// Release execution lock for a market.
    pub fn release_execution(&self, market_id: &str) {
        self.pending_executions.lock().remove(market_id);
    }
}
```

**Step 2: Use lock in orchestrator**

In `src/app/orchestrator.rs`, modify `handle_opportunity`:

```rust
fn handle_opportunity(
    opp: Opportunity,
    executor: Option<Arc<PolymarketExecutor>>,
    risk_manager: &RiskManager,
    notifiers: &Arc<NotifierRegistry>,
    state: &Arc<AppState>,
) {
    let market_id = opp.market_id().as_str();

    // Check for duplicate execution
    if !state.try_lock_execution(market_id) {
        debug!(market_id = %market_id, "Execution already in progress, skipping");
        return;
    }

    // ... rest of function
}
```

And in `spawn_execution`, release the lock after completion:

```rust
tokio::spawn(async move {
    let result = executor.execute_arbitrage(&opportunity).await;

    // Always release lock
    state.release_execution(&market_id);

    match result {
        // ... existing match arms
    }
});
```

**Step 3: Add test**

```rust
#[test]
fn test_execution_locking() {
    let state = AppState::default();

    assert!(state.try_lock_execution("market-1"));
    assert!(!state.try_lock_execution("market-1")); // Already locked

    state.release_execution("market-1");
    assert!(state.try_lock_execution("market-1")); // Can lock again
}
```

**Step 4: Run tests**

```bash
cargo test --all-features
```

**Step 5: Commit**

```bash
git add -A && git commit -m "feat: add execution locking to prevent duplicate trades"
```

---

### Task 1.3: Implement Partial Fill Recovery

When one leg fills and the other fails, attempt to cancel the filled leg or track as single-leg position.

**Files:**
- Modify: `src/adapter/polymarket/executor.rs`
- Modify: `src/app/orchestrator.rs`
- Modify: `src/domain/position.rs`

**Step 1: Add PositionStatus::PartialFill variant**

In `src/domain/position.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionStatus {
    Open,
    Closed,
    PartialFill, // Only one leg executed
}
```

**Step 2: Extend ArbitrageExecutionResult with order IDs**

In `src/adapter/polymarket/executor.rs`, update `PartialFill`:

```rust
PartialFill {
    filled_leg: TokenId,
    filled_order_id: String, // Add this
    failed_leg: TokenId,
    error: String,
}
```

Update the match arms in `execute_arbitrage` to include the order ID.

**Step 3: Handle partial fill in orchestrator**

In `spawn_execution`, add recovery logic:

```rust
ArbitrageExecutionResult::PartialFill { filled_leg, filled_order_id, failed_leg, error } => {
    warn!(
        filled_leg = %filled_leg,
        failed_leg = %failed_leg,
        error = %error,
        "Partial fill detected, attempting recovery"
    );

    // Try to cancel the filled order
    if let Err(cancel_err) = executor.cancel(&OrderId::new(&filled_order_id)).await {
        // Cancel failed - record as single-leg position for manual review
        warn!(error = %cancel_err, "Failed to cancel filled leg, recording partial position");
        record_partial_position(&state, &opportunity, &filled_leg);
    } else {
        info!("Successfully cancelled filled leg, no position recorded");
    }
}
```

**Step 4: Add record_partial_position function**

```rust
fn record_partial_position(state: &AppState, opportunity: &Opportunity, filled_leg: &TokenId) {
    use crate::domain::{Position, PositionLeg, PositionStatus};

    let (token, price) = if filled_leg == opportunity.yes_token() {
        (opportunity.yes_token().clone(), opportunity.yes_ask())
    } else {
        (opportunity.no_token().clone(), opportunity.no_ask())
    };

    let mut positions = state.positions_mut();
    let position = Position::new(
        positions.next_id(),
        opportunity.market_id().clone(),
        vec![PositionLeg::new(token, opportunity.volume(), price)],
        price * opportunity.volume(),
        opportunity.volume(),
        chrono::Utc::now(),
        PositionStatus::PartialFill,
    );
    positions.add(position);
}
```

**Step 5: Run tests**

```bash
cargo test --all-features
```

**Step 6: Commit**

```bash
git add -A && git commit -m "feat: implement partial fill recovery with cancel attempt"
```

---

## Part 2: Type System Improvements (MEDIUM Priority)

### Task 2.1: Rename Domain MarketInfo to Avoid Conflict

We have `domain::MarketInfo` and `exchange::MarketInfo`. Rename the domain one.

**Files:**
- Modify: `src/domain/market.rs`
- Modify: `src/domain/mod.rs`

**Step 1: Rename domain::MarketInfo to DomainMarket**

In `src/domain/market.rs`, rename:

```rust
/// Information about a market (domain layer).
#[derive(Debug, Clone)]
pub struct DomainMarket {
    id: MarketId,
    question: String,
    tokens: Vec<TokenInfo>,
}
```

Update the impl block accordingly.

**Step 2: Update re-exports in domain/mod.rs**

```rust
pub use market::{DomainMarket, MarketPair, TokenInfo};
```

**Step 3: Verify builds**

```bash
cargo build --all-features
```

**Step 4: Commit**

```bash
git add -A && git commit -m "refactor: rename domain::MarketInfo to DomainMarket"
```

---

### Task 2.2: Add Slippage Check Before Execution

Use the configured `max_slippage` to validate prices before submitting orders.

**Files:**
- Modify: `src/app/orchestrator.rs`
- Modify: `src/domain/orderbook.rs`

**Step 1: Add best_ask method to OrderBook**

In `src/domain/orderbook.rs`:

```rust
impl OrderBook {
    /// Get the best (lowest) ask price, if any.
    #[must_use]
    pub fn best_ask(&self) -> Option<Price> {
        self.asks.first().map(|level| level.price())
    }
}
```

**Step 2: Add slippage check in handle_opportunity**

In orchestrator, before execution:

```rust
fn check_slippage(
    opportunity: &Opportunity,
    cache: &OrderBookCache,
    max_slippage: Decimal,
) -> bool {
    // Get current best asks
    let yes_book = cache.get(opportunity.yes_token());
    let no_book = cache.get(opportunity.no_token());

    let (yes_current, no_current) = match (yes_book, no_book) {
        (Some(y), Some(n)) => (y.best_ask(), n.best_ask()),
        _ => return false, // Can't verify, reject
    };

    let (yes_current, no_current) = match (yes_current, no_current) {
        (Some(y), Some(n)) => (y, n),
        _ => return false,
    };

    // Check slippage for each leg
    let yes_slippage = (yes_current - opportunity.yes_ask()).abs() / opportunity.yes_ask();
    let no_slippage = (no_current - opportunity.no_ask()).abs() / opportunity.no_ask();

    yes_slippage <= max_slippage && no_slippage <= max_slippage
}
```

Call this before spawning execution.

**Step 3: Run tests**

```bash
cargo test --all-features
```

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: add pre-execution slippage check"
```

---

## Part 3: Dead Code Cleanup (LOW Priority)

### Task 3.1: Remove Dead Code Annotations

Remove `#[allow(dead_code)]` and `#[allow(unused_imports)]` where possible.

**Files:**
- Modify: `src/adapter/polymarket/types.rs`
- Modify: `src/adapter/polymarket/messages.rs`
- Modify: `src/adapter/polymarket/executor.rs`
- Modify: `src/adapter/polymarket/mod.rs`

**Step 1: Remove #[allow(dead_code)] from types.rs**

The `Market`, `Token`, `MarketsResponse` types are used. Remove the annotations and verify.

**Step 2: Remove #[allow(dead_code)] from messages.rs**

`BookMessage`, `PriceChangeMessage` are used. Remove annotations.

**Step 3: Remove unused import in executor.rs**

Line 12 has `#[allow(unused_imports)]` for `Signer`. Check if it's used, remove if not.

**Step 4: Remove unused import in mod.rs**

Line 15 has `#[allow(unused_imports)]` for `Token`. Check usage.

**Step 5: Verify builds**

```bash
cargo build --all-features
```

**Step 6: Commit**

```bash
git add -A && git commit -m "refactor: remove dead code annotations"
```

---

### Task 3.2: Remove Unused DomainMarket Type

After Task 2.1, if `DomainMarket` (formerly `MarketInfo`) is not used anywhere, remove it.

**Files:**
- Modify: `src/domain/market.rs`
- Modify: `src/domain/mod.rs`

**Step 1: Search for usages**

```bash
grep -r "DomainMarket" src/
```

**Step 2: If unused, remove the struct and its impl**

**Step 3: Update re-exports in mod.rs**

**Step 4: Verify builds**

```bash
cargo build --all-features
```

**Step 5: Commit**

```bash
git add -A && git commit -m "refactor: remove unused DomainMarket type"
```

---

## Part 4: Final Verification

### Task 4.1: Run Full Test Suite

**Step 1: Run all tests**

```bash
cargo test --all-features
```

**Step 2: Run clippy**

```bash
cargo clippy --all-features
```

**Step 3: Verify no warnings**

Expected: 0 warnings, all tests pass.

---

## Summary

| Task | Priority | Description |
|------|----------|-------------|
| 1.1 | HIGH | Implement order cancellation |
| 1.2 | HIGH | Add execution locking |
| 1.3 | HIGH | Partial fill recovery |
| 2.1 | MEDIUM | Rename domain MarketInfo |
| 2.2 | MEDIUM | Add slippage check |
| 3.1 | LOW | Remove dead code annotations |
| 3.2 | LOW | Remove unused types |
| 4.1 | - | Final verification |
