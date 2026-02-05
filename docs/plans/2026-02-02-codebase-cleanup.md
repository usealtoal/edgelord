# Codebase Cleanup Implementation Plan

> Status: Historical
> Superseded by: N/A
> Summary:
> - Goal: Address all audit findings - dead code removal/implementation, pattern consistency, documentation gaps, test coverage, and clippy pedantic/nursery warnings.
> - Scope: Task 1.1: Remove Deprecated detector.rs Module
> Planned Outcomes:
> - Task 1.1: Remove Deprecated detector.rs Module
> - Task 1.2: Remove Unused money::constants Module


> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Address all audit findings - dead code removal/implementation, pattern consistency, documentation gaps, test coverage, and clippy pedantic/nursery warnings.

**Architecture:** Systematic cleanup in phases: (1) Dead code - fully implement or remove, (2) Pattern consistency fixes, (3) Documentation, (4) Test coverage, (5) Clippy warnings. Each phase commits atomically.

**Tech Stack:** Rust 2021, cargo clippy, cargo test

---

## Part 1: Dead Code Resolution

### Task 1.1: Remove Deprecated detector.rs Module

The `src/domain/detector.rs` module is deprecated - it only re-exports from `strategy::single_condition`. Remove it and update imports.

**Files:**
- Delete: `src/domain/detector.rs`
- Modify: `src/domain/mod.rs`

**Step 1: Update domain/mod.rs to remove detector**

Remove the `mod detector` declaration and the re-export. Change:
```rust
mod detector;
```
to nothing (delete the line).

And change:
```rust
// Detector
pub use detector::{detect_single_condition, DetectorConfig};
```
to:
```rust
// Detector (re-exported from strategy for convenience)
pub use strategy::single_condition::{detect_single_condition, SingleConditionConfig as DetectorConfig};
```

**Step 2: Delete the file**

```bash
rm src/domain/detector.rs
```

**Step 3: Verify tests pass**

```bash
cargo test --all-features
```

**Step 4: Commit**

```bash
git add -A && git commit -m "refactor: remove deprecated detector.rs module"
```

---

### Task 1.2: Remove Unused money::constants Module

The `constants` module in `money.rs` is never used outside tests. Remove it.

**Files:**
- Modify: `src/domain/money.rs`

**Step 1: Remove the constants module**

Delete lines 11-21 (the `#[allow(unused)]` and entire `constants` module):
```rust
/// Common monetary constants.
#[allow(unused)]
pub mod constants {
    use rust_decimal::Decimal;

    /// One dollar.
    pub const ONE_DOLLAR: Decimal = Decimal::ONE;

    /// Zero dollars.
    pub const ZERO: Decimal = Decimal::ZERO;
}
```

**Step 2: Update tests to use Decimal directly**

Change the test:
```rust
#[test]
fn constants_are_correct() {
    assert_eq!(constants::ONE_DOLLAR, Decimal::ONE);
    assert_eq!(constants::ZERO, Decimal::ZERO);
}
```
to be removed entirely (delete the test).

**Step 3: Verify tests pass**

```bash
cargo test --all-features
```

**Step 4: Commit**

```bash
git add -A && git commit -m "refactor: remove unused money::constants module"
```

---

### Task 1.3: Remove Dead log_market Function

The `log_market` function in orchestrator.rs is never called - the same code is inlined. Remove it.

**Files:**
- Modify: `src/app/orchestrator.rs`

**Step 1: Remove the function**

Delete lines 332-340:
```rust
/// Log market pair being tracked.
#[allow(dead_code)]
fn log_market(pair: &MarketPair) {
    debug!(
        market_id = %pair.market_id(),
        question = %pair.question(),
        "Tracking market"
    );
}
```

**Step 2: Verify tests pass**

```bash
cargo test --all-features
```

**Step 3: Commit**

```bash
git add -A && git commit -m "refactor: remove unused log_market function"
```

---

### Task 1.4: Handle PriceChange Messages or Document Intentional Skip

Currently `WsMessage::PriceChange(_) => {}` silently ignores price changes. Either implement handling or document why we skip them.

**Files:**
- Modify: `src/app/orchestrator.rs`

**Step 1: Add documentation comment explaining the skip**

Change:
```rust
WsMessage::PriceChange(_) => {}
```
to:
```rust
// Price changes are incremental updates; we only need full book snapshots
// for arbitrage detection since we need both bid and ask sides
WsMessage::PriceChange(_) => {}
```

**Step 2: Commit**

```bash
git add -A && git commit -m "docs: explain why PriceChange messages are skipped"
```

---

### Task 1.5: Remove Unused ExchangeClient Trait

The `ExchangeClient` trait is defined but never implemented. It was forward-looking for multi-exchange support. Remove it - we can add it back when actually needed (YAGNI).

**Files:**
- Modify: `src/exchange/traits.rs`
- Modify: `src/exchange/mod.rs`

**Step 1: Remove ExchangeClient trait from traits.rs**

Delete lines 80-124 (the `MarketInfo` struct and `ExchangeClient` trait):
```rust
/// Represents market information from an exchange.
///
/// This is a simplified market info type for the exchange abstraction layer.
/// Exchange implementations can convert their specific market types to this.
#[derive(Debug, Clone)]
pub struct MarketInfo {
    /// Unique identifier for the market.
    pub id: String,
    /// Human-readable market name/question.
    pub name: String,
    /// Whether the market is active for trading.
    pub active: bool,
}

/// Client for fetching market data from an exchange.
#[async_trait]
pub trait ExchangeClient: Send + Sync {
    /// Fetch all available markets from the exchange.
    async fn get_markets(&self) -> Result<Vec<MarketInfo>, Error>;

    /// Get the exchange name for logging/debugging.
    fn exchange_name(&self) -> &'static str;
}
```

**Step 2: Update exchange/mod.rs exports**

Change:
```rust
pub use traits::{ExchangeClient, ExecutionResult, MarketInfo, OrderExecutor, OrderId, OrderRequest, OrderSide};
```
to:
```rust
pub use traits::{ExecutionResult, OrderExecutor, OrderId, OrderRequest, OrderSide};
```

**Step 3: Verify nothing uses ExchangeClient**

```bash
cargo build --all-features
```

**Step 4: Commit**

```bash
git add -A && git commit -m "refactor: remove unused ExchangeClient trait (YAGNI)"
```

---

### Task 1.6: Implement MarketRebalancingStrategy::detect()

The strategy's `detect()` method is a stub. Implement it properly using the existing `detect_rebalancing()` function and extending `DetectionContext` to support multi-outcome markets.

**Files:**
- Modify: `src/domain/strategy/context.rs`
- Modify: `src/domain/strategy/market_rebalancing.rs`
- Modify: `src/adapter/polymarket/registry.rs`
- Modify: `src/adapter/polymarket/mod.rs`
- Modify: `src/app/orchestrator.rs`

**Step 1: Extend DetectionContext with token list**

In `src/domain/strategy/context.rs`, add a field for multi-outcome token lists:

Add after line 74 (`market_ctx: MarketContext,`):
```rust
    /// Token IDs for multi-outcome markets (empty for binary).
    token_ids: Vec<crate::domain::TokenId>,
```

Update `new()` constructor:
```rust
    pub fn new(pair: &'a MarketPair, cache: &'a OrderBookCache) -> Self {
        Self {
            pair,
            cache,
            market_ctx: MarketContext::binary(),
            token_ids: vec![],
        }
    }
```

Add a new constructor for multi-outcome:
```rust
    /// Create context for a multi-outcome market.
    pub fn multi_outcome(
        pair: &'a MarketPair,
        cache: &'a OrderBookCache,
        token_ids: Vec<crate::domain::TokenId>,
    ) -> Self {
        Self {
            pair,
            cache,
            market_ctx: MarketContext::multi_outcome(token_ids.len()),
            token_ids,
        }
    }

    /// Get token IDs for multi-outcome detection.
    pub fn token_ids(&self) -> &[crate::domain::TokenId] {
        &self.token_ids
    }
```

**Step 2: Implement MarketRebalancingStrategy::detect()**

In `src/domain/strategy/market_rebalancing.rs`, replace the stub:

```rust
fn detect(&self, ctx: &DetectionContext) -> Vec<Opportunity> {
    let token_ids = ctx.token_ids();

    // Need at least 3 outcomes for rebalancing (binary handled by single_condition)
    if token_ids.len() < 3 {
        return vec![];
    }

    // Use the existing detection function
    if let Some(rebal_opp) = detect_rebalancing(
        ctx.pair.market_id(),
        ctx.pair.question(),
        token_ids,
        ctx.cache,
        &self.config,
    ) {
        // Convert RebalancingOpportunity to the standard Opportunity type
        // For now, we use the first two legs as YES/NO (simplified)
        // TODO: Create a proper multi-leg Opportunity variant
        if rebal_opp.legs.len() >= 2 {
            if let Ok(opp) = Opportunity::builder()
                .market_id(rebal_opp.market_id.clone())
                .question(&rebal_opp.question)
                .yes_token(rebal_opp.legs[0].token_id.clone(), rebal_opp.legs[0].price)
                .no_token(rebal_opp.legs[1].token_id.clone(), rebal_opp.legs[1].price)
                .volume(rebal_opp.volume)
                .build()
            {
                return vec![opp];
            }
        }
    }

    vec![]
}
```

**Step 3: Remove #[allow(dead_code)] from detect_rebalancing**

Change line 154:
```rust
#[allow(dead_code)] // Used in tests; may be useful as a standalone utility
pub fn detect_rebalancing(
```
to:
```rust
/// Detect rebalancing opportunity across multiple outcomes.
pub fn detect_rebalancing(
```

**Step 4: Verify tests pass**

```bash
cargo test --all-features
```

**Step 5: Commit**

```bash
git add -A && git commit -m "feat: implement MarketRebalancingStrategy::detect()"
```

---

## Part 2: Pattern Consistency

### Task 2.1: Make market_rebalancing Module Public

For consistency with `single_condition` and `combinatorial`, make the module public.

**Files:**
- Modify: `src/domain/strategy/mod.rs`

**Step 1: Change module visibility**

Change line 32:
```rust
mod market_rebalancing;
```
to:
```rust
pub mod market_rebalancing;
```

**Step 2: Verify builds**

```bash
cargo build --all-features
```

**Step 3: Commit**

```bash
git add -A && git commit -m "refactor: make market_rebalancing module public for consistency"
```

---

### Task 2.2: Standardize Event Constructors to From Trait

`RiskEvent::new()` should be `impl From<(&str, &RiskError)>` for consistency with `OpportunityEvent`.

**Files:**
- Modify: `src/service/notifier.rs`

**Step 1: Replace RiskEvent::new with From impl**

Replace lines 86-93:
```rust
impl RiskEvent {
    pub fn new(market_id: &str, error: &RiskError) -> Self {
        Self {
            market_id: market_id.to_string(),
            reason: error.to_string(),
        }
    }
}
```
with:
```rust
impl RiskEvent {
    /// Create a new risk rejection event.
    #[must_use]
    pub fn new(market_id: &str, error: &RiskError) -> Self {
        Self {
            market_id: market_id.to_string(),
            reason: error.to_string(),
        }
    }
}
```

Actually, keep it as `new()` since the signature `(&str, &RiskError)` doesn't map cleanly to a single `From` source type. Just add `#[must_use]`.

**Step 2: Verify builds**

```bash
cargo build --all-features
```

**Step 3: Commit**

```bash
git add -A && git commit -m "refactor: add #[must_use] to RiskEvent::new"
```

---

## Part 3: Documentation

### Task 3.1: Add Module Doc to config.rs

**Files:**
- Modify: `src/app/config.rs`

**Step 1: Add module documentation at the top**

Add before line 1:
```rust
//! Application configuration loading and validation.
//!
//! Configuration is loaded from a TOML file with environment variable overrides
//! for sensitive values like `WALLET_PRIVATE_KEY`.

```

**Step 2: Commit**

```bash
git add -A && git commit -m "docs: add module documentation to config.rs"
```

---

### Task 3.2: Document Frank-Wolfe Algorithm Steps

**Files:**
- Modify: `src/domain/strategy/combinatorial/frank_wolfe.rs`

**Step 1: Add inline comments to the project() method**

Find the `project()` method and add step-by-step comments explaining:
1. Initialize with current prices
2. Compute Bregman gradient
3. Solve ILP oracle to find minimizing vertex
4. Update toward that vertex with step size
5. Check convergence via duality gap

(This requires reading the file to add appropriate comments - implementation detail for the executor)

**Step 2: Commit**

```bash
git add -A && git commit -m "docs: add inline comments explaining Frank-Wolfe algorithm"
```

---

### Task 3.3: Document WebSocket Message Handling

**Files:**
- Modify: `src/adapter/polymarket/websocket.rs`

**Step 1: Add comments explaining the message loop, ping/pong, and error handling**

(Implementation detail - add comments to the main loop explaining connection lifecycle)

**Step 2: Commit**

```bash
git add -A && git commit -m "docs: add WebSocket message handling documentation"
```

---

## Part 4: Test Coverage

### Task 4.1: Add TelegramConfig::from_env() Tests

**Files:**
- Modify: `src/service/telegram.rs`

**Step 1: Add comprehensive tests**

Add to the tests module:
```rust
#[test]
fn test_from_env_missing_token() {
    // Clear env vars
    std::env::remove_var("TELEGRAM_BOT_TOKEN");
    std::env::remove_var("TELEGRAM_CHAT_ID");

    assert!(TelegramConfig::from_env().is_none());
}

#[test]
fn test_from_env_missing_chat_id() {
    std::env::set_var("TELEGRAM_BOT_TOKEN", "test-token");
    std::env::remove_var("TELEGRAM_CHAT_ID");

    let result = TelegramConfig::from_env();
    assert!(result.is_none());

    // Cleanup
    std::env::remove_var("TELEGRAM_BOT_TOKEN");
}

#[test]
fn test_from_env_invalid_chat_id() {
    std::env::set_var("TELEGRAM_BOT_TOKEN", "test-token");
    std::env::set_var("TELEGRAM_CHAT_ID", "not-a-number");

    let result = TelegramConfig::from_env();
    assert!(result.is_none());

    // Cleanup
    std::env::remove_var("TELEGRAM_BOT_TOKEN");
    std::env::remove_var("TELEGRAM_CHAT_ID");
}

#[test]
fn test_from_env_valid() {
    std::env::set_var("TELEGRAM_BOT_TOKEN", "test-token");
    std::env::set_var("TELEGRAM_CHAT_ID", "12345");
    std::env::set_var("TELEGRAM_NOTIFY_OPPORTUNITIES", "true");

    let config = TelegramConfig::from_env().unwrap();
    assert_eq!(config.bot_token, "test-token");
    assert_eq!(config.chat_id, 12345);
    assert!(config.notify_opportunities);
    assert!(config.notify_executions);
    assert!(config.notify_risk_rejections);

    // Cleanup
    std::env::remove_var("TELEGRAM_BOT_TOKEN");
    std::env::remove_var("TELEGRAM_CHAT_ID");
    std::env::remove_var("TELEGRAM_NOTIFY_OPPORTUNITIES");
}

#[test]
fn test_escape_markdown_all_special_chars() {
    let special = "_*[]()~`>#+-=|{}.!";
    let escaped = escape_markdown(special);
    assert_eq!(escaped, "\\_\\*\\[\\]\\(\\)\\~\\`\\>\\#\\+\\-\\=\\|\\{\\}\\.\\!");
}

#[test]
fn test_escape_markdown_empty() {
    assert_eq!(escape_markdown(""), "");
}
```

**Step 2: Run tests**

```bash
cargo test --all-features telegram
```

**Step 3: Commit**

```bash
git add -A && git commit -m "test: add TelegramConfig::from_env() edge case tests"
```

---

### Task 4.2: Add Position Limit Test for RiskManager

**Files:**
- Modify: `src/service/risk.rs`

**Step 1: Add test for position limit**

Add to tests module:
```rust
#[test]
fn test_check_position_limit() {
    use crate::domain::{Position, PositionLeg, PositionStatus};

    let limits = RiskLimits {
        max_position_per_market: dec!(100), // Only $100 per market
        min_profit_threshold: dec!(0),
        ..Default::default()
    };
    let state = Arc::new(AppState::new(limits));

    // Add existing position in this market
    {
        let mut positions = state.positions_mut();
        let position = Position::new(
            positions.next_id(),
            MarketId::from("test-market"),
            vec![
                PositionLeg::new(TokenId::from("yes"), dec!(50), dec!(0.45)),
                PositionLeg::new(TokenId::from("no"), dec!(50), dec!(0.45)),
            ],
            dec!(45), // $45 entry cost
            dec!(50),
            chrono::Utc::now(),
            PositionStatus::Open,
        );
        positions.add(position);
    }

    let risk = RiskManager::new(state);

    // Try to add $90 more (45 + 81 = 126 > 100 limit)
    let opp = make_opportunity(dec!(100), dec!(0.45), dec!(0.45)); // $90 cost
    let result = risk.check(&opp);

    assert!(!result.is_approved());
    assert!(matches!(
        result.rejection_error(),
        Some(RiskError::PositionLimitExceeded { .. })
    ));
}
```

**Step 2: Run tests**

```bash
cargo test --all-features risk
```

**Step 3: Commit**

```bash
git add -A && git commit -m "test: add position limit check test for RiskManager"
```

---

### Task 4.3: Add AppState total_exposure Test

**Files:**
- Modify: `src/app/state.rs`

**Step 1: Add test**

Add to tests module:
```rust
#[test]
fn test_total_exposure() {
    use crate::domain::{MarketId, Position, PositionLeg, PositionStatus, TokenId};
    use rust_decimal_macros::dec;

    let state = AppState::default();

    // Initially zero
    assert_eq!(state.total_exposure(), Decimal::ZERO);

    // Add a position
    {
        let mut positions = state.positions_mut();
        let position = Position::new(
            positions.next_id(),
            MarketId::from("test"),
            vec![
                PositionLeg::new(TokenId::from("yes"), dec!(100), dec!(0.45)),
                PositionLeg::new(TokenId::from("no"), dec!(100), dec!(0.45)),
            ],
            dec!(90), // entry cost
            dec!(100),
            chrono::Utc::now(),
            PositionStatus::Open,
        );
        positions.add(position);
    }

    // Now has exposure
    assert_eq!(state.total_exposure(), dec!(90));
}
```

**Step 2: Run tests**

```bash
cargo test --all-features state
```

**Step 3: Commit**

```bash
git add -A && git commit -m "test: add total_exposure test for AppState"
```

---

### Task 4.4: Add MarketRegistry Tests

**Files:**
- Modify: `src/adapter/polymarket/registry.rs`

**Step 1: Add test module**

Add at the end of the file:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::polymarket::types::Token;

    fn make_market(condition_id: &str, question: &str, yes_id: &str, no_id: &str) -> Market {
        Market {
            condition_id: condition_id.to_string(),
            question: Some(question.to_string()),
            tokens: vec![
                Token {
                    token_id: yes_id.to_string(),
                    outcome: "Yes".to_string(),
                    price: Some("0.5".to_string()),
                },
                Token {
                    token_id: no_id.to_string(),
                    outcome: "No".to_string(),
                    price: Some("0.5".to_string()),
                },
            ],
            active: true,
            closed: false,
        }
    }

    #[test]
    fn test_registry_new() {
        let registry = MarketRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_from_markets() {
        let markets = vec![
            make_market("cond-1", "Question 1?", "yes-1", "no-1"),
            make_market("cond-2", "Question 2?", "yes-2", "no-2"),
        ];

        let registry = MarketRegistry::from_markets(&markets);

        assert_eq!(registry.len(), 2);
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_registry_skips_non_binary() {
        let mut market = make_market("cond-1", "Multi?", "a", "b");
        market.tokens.push(Token {
            token_id: "c".to_string(),
            outcome: "Maybe".to_string(),
            price: None,
        });

        let registry = MarketRegistry::from_markets(&[market]);

        assert!(registry.is_empty());
    }

    #[test]
    fn test_get_market_for_token() {
        let markets = vec![make_market("cond-1", "Q?", "yes-1", "no-1")];
        let registry = MarketRegistry::from_markets(&markets);

        let yes_token = TokenId::from("yes-1");
        let no_token = TokenId::from("no-1");
        let unknown = TokenId::from("unknown");

        assert!(registry.get_market_for_token(&yes_token).is_some());
        assert!(registry.get_market_for_token(&no_token).is_some());
        assert!(registry.get_market_for_token(&unknown).is_none());

        // Both tokens map to same market
        let yes_market = registry.get_market_for_token(&yes_token).unwrap();
        let no_market = registry.get_market_for_token(&no_token).unwrap();
        assert_eq!(yes_market.market_id(), no_market.market_id());
    }
}
```

**Step 2: Run tests**

```bash
cargo test --all-features registry
```

**Step 3: Commit**

```bash
git add -A && git commit -m "test: add MarketRegistry unit tests"
```

---

## Part 5: Clippy Pedantic/Nursery Fixes

### Task 5.1: Fix #[must_use] Warnings

Add `#[must_use]` to all accessor methods that return values. This is the largest category (~100 warnings).

**Files:** Multiple files across the codebase

**Step 1: Run clippy and fix iteratively**

```bash
cargo clippy --all-features --fix --allow-dirty -- -W clippy::must_use_candidate
```

If that doesn't work automatically, manually add `#[must_use]` to methods like:
- `as_str()` methods on ID types
- `is_*()` boolean methods
- Getter methods that return references

**Step 2: Verify**

```bash
cargo clippy --all-features -- -W clippy::must_use_candidate
```

**Step 3: Commit**

```bash
git add -A && git commit -m "fix: add #[must_use] attributes per clippy pedantic"
```

---

### Task 5.2: Fix Doc Markdown Backticks

Add backticks around type names in doc comments (e.g., `TokenId` not TokenId).

**Files:** Multiple files

**Step 1: Run clippy fix**

```bash
cargo clippy --all-features --fix --allow-dirty -- -W clippy::doc_markdown
```

**Step 2: Verify**

```bash
cargo clippy --all-features -- -W clippy::doc_markdown
```

**Step 3: Commit**

```bash
git add -A && git commit -m "docs: add backticks around type names in doc comments"
```

---

### Task 5.3: Fix const fn Candidates

Mark pure functions as `const fn` where possible.

**Files:** Multiple files

**Step 1: Run clippy and identify candidates**

```bash
cargo clippy --all-features -- -W clippy::missing_const_for_fn 2>&1 | grep "this could be"
```

**Step 2: Add `const` to identified functions**

For each identified function, change `pub fn` to `pub const fn` if it:
- Only does const-compatible operations
- Doesn't call non-const functions
- Doesn't use mutable references

**Step 3: Verify**

```bash
cargo clippy --all-features -- -W clippy::missing_const_for_fn
```

**Step 4: Commit**

```bash
git add -A && git commit -m "perf: mark pure functions as const fn"
```

---

### Task 5.4: Fix Remaining Clippy Warnings

Address any remaining pedantic/nursery warnings.

**Step 1: Run full clippy check**

```bash
cargo clippy --all-features -- -W clippy::pedantic -W clippy::nursery 2>&1 | grep "^warning"
```

**Step 2: Fix each warning category systematically**

Common fixes:
- `similar_names`: Rename variables or add `#[allow(clippy::similar_names)]` with justification
- `too_many_arguments`: Already allowed in orchestrator.rs, add to other functions if needed
- `module_name_repetitions`: Consider renaming or allowing

**Step 3: Verify clean**

```bash
cargo clippy --all-features -- -W clippy::pedantic -W clippy::nursery
```

**Step 4: Commit**

```bash
git add -A && git commit -m "fix: address remaining clippy pedantic/nursery warnings"
```

---

## Part 6: Final Verification

### Task 6.1: Full Test Suite

**Step 1: Run all tests**

```bash
cargo test --all-features
```

**Step 2: Run clippy clean check**

```bash
cargo clippy --all-features -- -D warnings -W clippy::pedantic -W clippy::nursery
```

**Step 3: Verify no dead code warnings**

```bash
cargo build --all-features 2>&1 | grep "warning.*dead_code"
```

Should return nothing.

**Step 4: Final commit if any fixups needed**

```bash
git add -A && git commit -m "chore: final cleanup verification"
```

---

## Summary

| Part | Tasks | Focus |
|------|-------|-------|
| 1 | 1.1-1.6 | Dead code removal/implementation |
| 2 | 2.1-2.2 | Pattern consistency |
| 3 | 3.1-3.3 | Documentation |
| 4 | 4.1-4.4 | Test coverage |
| 5 | 5.1-5.4 | Clippy pedantic/nursery |
| 6 | 6.1 | Final verification |

**Total: ~20 tasks**

**Commit style:** One-liner messages, no Claude authorship.
