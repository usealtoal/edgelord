# Production Hardening Implementation Plan

> Status: Historical
> Superseded by: N/A
> Summary:
> - Goal: Make edgelord production-ready with strict invariants, typed errors, and reliable execution under failure.
> - Scope: Task 0.1: Partial-fill recovery policy in executor + orchestrator
> Planned Outcomes:
> - Task 0.1: Partial-fill recovery policy in executor + orchestrator
> - Task 0.2: Execution timeouts and lock release on panic

#
> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.
#
**Goal:** Make edgelord production-ready with strict invariants, typed errors, and reliable execution under failure.
#
**Architecture:** Apply clean-break, risk-ordered remediation. Phase 0 fixes execution/risk/data integrity blockers. Phase 1 enforces domain invariants and error discipline. Phase 2 adds reliability/ops (timeouts, retries, shutdown). Phase 3 normalizes config/CLI with strict validation and defaults. All changes are TDD-first with small, verifiable steps.
#
**Tech Stack:** Rust, Tokio, Diesel (SQLite), tracing, reqwest, tokio-tungstenite, clap.
#
---
#
## Phase 0: Production Blockers (Execution, Risk, Data Integrity)
#
### Task 0.1: Partial-fill recovery policy in executor + orchestrator
#
**Files:**
- Modify: `src/core/exchange/polymarket/executor.rs`
- Modify: `src/app/orchestrator/execution.rs`
- Test: `tests/execution_tests.rs`
#
**Step 1: Write failing test for partial fill rollback**
#
```rust
#[tokio::test]
async fn partial_fill_triggers_cancel_and_records_partial_position_on_failure() {
    // Arrange: executor returns PartialFill, cancel fails on one leg
    // Assert: position recorded with PartialFill status and missing leg ids
}
```
#
**Step 2: Run test to verify it fails**
#
Run: `cargo test tests::execution_tests::partial_fill_triggers_cancel_and_records_partial_position_on_failure -v`
Expected: FAIL with "test not implemented" or missing behavior.
#
**Step 3: Implement minimal behavior**
#
```rust
// In executor: return PartialFill with filled/failed legs
// In orchestrator: attempt cancel; if any cancel fails -> record_partial_position
```
#
**Step 4: Run test to verify it passes**
#
Run: `cargo test tests::execution_tests::partial_fill_triggers_cancel_and_records_partial_position_on_failure -v`
Expected: PASS
#
**Step 5: Commit**
#
```bash
git add tests/execution_tests.rs src/core/exchange/polymarket/executor.rs src/app/orchestrator/execution.rs
git commit -m "fix(execution): handle partial fills deterministically"
```
#
### Task 0.2: Execution timeouts and lock release on panic
#
**Files:**
- Modify: `src/app/orchestrator/execution.rs`
- Test: `tests/execution_tests.rs`
#
**Step 1: Write failing test for timeout release**
#
```rust
#[tokio::test]
async fn execution_timeout_releases_lock() {
    // Arrange: executor never returns
    // Assert: lock released after timeout and error event emitted
}
```
#
**Step 2: Run test to verify it fails**
#
Run: `cargo test tests::execution_tests::execution_timeout_releases_lock -v`
Expected: FAIL
#
**Step 3: Implement timeout + guard**
#
```rust
// Wrap executor.execute_arbitrage in tokio::time::timeout
// Use guard to release lock in Drop or finally block
```
#
**Step 4: Run test to verify it passes**
#
Run: `cargo test tests::execution_tests::execution_timeout_releases_lock -v`
Expected: PASS
#
**Step 5: Commit**
#
```bash
git add tests/execution_tests.rs src/app/orchestrator/execution.rs
git commit -m "fix(app): add execution timeout and lock guard"
```
#
### Task 0.3: Atomic exposure reservation in risk manager
#
**Files:**
- Modify: `src/app/state.rs`
- Modify: `src/core/service/risk.rs`
- Test: `tests/risk_tests.rs`
#
**Step 1: Write failing test for concurrent exposure**
#
```rust
#[tokio::test]
async fn concurrent_opportunities_cannot_exceed_total_exposure() {
    // Arrange: two opportunities that together exceed limit
    // Assert: only one is approved/reserved
}
```
#
**Step 2: Run test to verify it fails**
#
Run: `cargo test tests::risk_tests::concurrent_opportunities_cannot_exceed_total_exposure -v`
Expected: FAIL
#
**Step 3: Implement reservation**
#
```rust
// Add pending exposure tracking to AppState
// RiskManager check reserves exposure on approval
```
#
**Step 4: Run test to verify it passes**
#
Run: `cargo test tests::risk_tests::concurrent_opportunities_cannot_exceed_total_exposure -v`
Expected: PASS
#
**Step 5: Commit**
#
```bash
git add tests/risk_tests.rs src/app/state.rs src/core/service/risk.rs
git commit -m "fix(risk): reserve exposure atomically"
```
#
### Task 0.4: Stats integrity (IDs + daily stats transaction)
#
**Files:**
- Modify: `src/core/service/stats/mod.rs`
- Modify: `src/core/db/mod.rs`
- Test: `tests/stats_tests.rs`
#
**Step 1: Write failing test for concurrent inserts**
#
```rust
#[tokio::test]
async fn record_opportunity_returns_correct_id_under_concurrency() {
    // Arrange: concurrent inserts
    // Assert: returned IDs match rows inserted
}
```
#
**Step 2: Run test to verify it fails**
#
Run: `cargo test tests::stats_tests::record_opportunity_returns_correct_id_under_concurrency -v`
Expected: FAIL
#
**Step 3: Implement transaction + last_insert_rowid**
#
```rust
// Use Diesel transaction for insert + daily stats update
// Use last_insert_rowid() for ID
```
#
**Step 4: Run test to verify it passes**
#
Run: `cargo test tests::stats_tests::record_opportunity_returns_correct_id_under_concurrency -v`
Expected: PASS
#
**Step 5: Commit**
#
```bash
git add tests/stats_tests.rs src/core/service/stats/mod.rs src/core/db/mod.rs
git commit -m "fix(stats): transactional inserts and correct IDs"
```
#
### Task 0.5: Fail fast on JSON serialization in SQLite store
#
**Files:**
- Modify: `src/core/store/sqlite.rs`
- Test: `src/core/store/sqlite.rs` (unit test)
#
**Step 1: Write failing test for invalid JSON**
#
```rust
#[tokio::test]
async fn save_returns_error_on_invalid_json() {
    // Arrange: relation with invalid data triggering serde error
    // Assert: save returns Err
}
```
#
**Step 2: Run test to verify it fails**
#
Run: `cargo test sqlite_relation_roundtrip save_returns_error_on_invalid_json -v`
Expected: FAIL
#
**Step 3: Implement error propagation**
#
```rust
// Replace unwrap_or_default with ? and Error::Parse
```
#
**Step 4: Run test to verify it passes**
#
Run: `cargo test sqlite_relation_roundtrip save_returns_error_on_invalid_json -v`
Expected: PASS
#
**Step 5: Commit**
#
```bash
git add src/core/store/sqlite.rs
git commit -m "fix(store): fail fast on JSON serialization"
```
#
---
#
## Phase 1: Domain Invariants + Error Discipline
#
### Task 1.1: Domain invariant constructors (Opportunity, Position, Market)
#
**Files:**
- Modify: `src/core/domain/opportunity.rs`
- Modify: `src/core/domain/position.rs`
- Modify: `src/core/domain/market.rs`
- Test: `src/core/domain/opportunity.rs` (unit tests)
- Test: `src/core/domain/position.rs` (unit tests)
- Test: `src/core/domain/market.rs` (unit tests)
#
**Step 1: Write failing tests for invalid inputs**
#
```rust
#[test]
fn opportunity_rejects_non_positive_volume() { /* ... */ }
#[test]
fn opportunity_rejects_payout_not_greater_than_cost() { /* ... */ }
```
#
**Step 2: Run tests to verify failure**
#
Run: `cargo test opportunity_rejects_non_positive_volume opportunity_rejects_payout_not_greater_than_cost -v`
Expected: FAIL
#
**Step 3: Implement Result-returning constructors**
#
```rust
// Opportunity::try_new(...) -> Result<Opportunity, DomainError>
// Position::try_new(...)
// Market::try_new(...)
```
#
**Step 4: Run tests to verify pass**
#
Run: `cargo test opportunity_rejects_non_positive_volume opportunity_rejects_payout_not_greater_than_cost -v`
Expected: PASS
#
**Step 5: Commit**
#
```bash
git add src/core/domain/opportunity.rs src/core/domain/position.rs src/core/domain/market.rs
git commit -m "refactor(domain): enforce invariants via constructors"
```
#
### Task 1.2: Strategy input validation + fail-closed behavior
#
**Files:**
- Modify: `src/core/strategy/context.rs`
- Modify: `src/core/strategy/condition/single.rs`
- Modify: `src/core/strategy/rebalancing/mod.rs`
- Test: `tests/strategy_tests.rs`
#
**Step 1: Add failing tests for missing books**
#
```rust
#[test]
fn strategy_skips_when_order_books_missing() { /* ... */ }
```
#
**Step 2: Run tests to verify failure**
#
Run: `cargo test tests::strategy_tests::strategy_skips_when_order_books_missing -v`
Expected: FAIL
#
**Step 3: Implement fail-closed checks**
#
```rust
// Return empty opportunities if any required book missing
```
#
**Step 4: Run tests to verify pass**
#
Run: `cargo test tests::strategy_tests::strategy_skips_when_order_books_missing -v`
Expected: PASS
#
**Step 5: Commit**
#
```bash
git add src/core/strategy/context.rs src/core/strategy/condition/single.rs src/core/strategy/rebalancing/mod.rs tests/strategy_tests.rs
git commit -m "fix(strategy): fail closed on missing order books"
```
#
### Task 1.3: Remove panic paths in exchange factory + approval
#
**Files:**
- Modify: `src/core/exchange/factory.rs`
- Modify: `src/core/exchange/polymarket/approval.rs`
- Test: `tests/exchange_tests.rs`
#
**Step 1: Add failing tests for missing config**
#
```rust
#[test]
fn factory_returns_error_when_exchange_config_missing() { /* ... */ }
```
#
**Step 2: Run tests to verify failure**
#
Run: `cargo test tests::exchange_tests::factory_returns_error_when_exchange_config_missing -v`
Expected: FAIL
#
**Step 3: Replace unwrap/expect with typed errors**
#
```rust
// Return Result with ConfigError::MissingField
```
#
**Step 4: Run tests to verify pass**
#
Run: `cargo test tests::exchange_tests::factory_returns_error_when_exchange_config_missing -v`
Expected: PASS
#
**Step 5: Commit**
#
```bash
git add src/core/exchange/factory.rs src/core/exchange/polymarket/approval.rs tests/exchange_tests.rs
git commit -m "fix(exchange): remove panic paths in factory/approval"
```
#
### Task 1.4: Subscription manager lock safety
#
**Files:**
- Modify: `src/core/service/subscription/priority.rs`
- Test: `src/core/service/subscription/priority.rs` (unit tests)
#
**Step 1: Add failing test for poisoned lock handling**
#
```rust
#[test]
fn lock_poisoning_does_not_panic() { /* ... */ }
```
#
**Step 2: Run tests to verify failure**
#
Run: `cargo test lock_poisoning_does_not_panic -v`
Expected: FAIL
#
**Step 3: Replace expect with error handling**
#
```rust
// Map poisoned lock to Result error
```
#
**Step 4: Run tests to verify pass**
#
Run: `cargo test lock_poisoning_does_not_panic -v`
Expected: PASS
#
**Step 5: Commit**
#
```bash
git add src/core/service/subscription/priority.rs
git commit -m "fix(service): handle poisoned locks safely"
```
#
---
#
## Phase 2: Reliability & Ops Hardening
#
### Task 2.1: WebSocket reconnect resilience
#
**Files:**
- Modify: `src/core/exchange/reconnecting.rs`
- Modify: `src/core/exchange/polymarket/websocket.rs`
- Test: `tests/exchange_tests.rs`
#
**Step 1: Failing reconnect test**
#
```rust
#[tokio::test]
async fn reconnect_retries_on_subscribe_failure() { /* ... */ }
```
#
**Step 2: Run test to verify failure**
#
Run: `cargo test tests::exchange_tests::reconnect_retries_on_subscribe_failure -v`
Expected: FAIL
#
**Step 3: Implement retry with jitter**
#
```rust
// Retry subscribe with backoff + jitter
```
#
**Step 4: Run test to verify pass**
#
Run: `cargo test tests::exchange_tests::reconnect_retries_on_subscribe_failure -v`
Expected: PASS
#
**Step 5: Commit**
#
```bash
git add src/core/exchange/reconnecting.rs src/core/exchange/polymarket/websocket.rs tests/exchange_tests.rs
git commit -m "fix(exchange): resilient reconnect with jitter"
```
#
### Task 2.2: HTTP client timeouts and retries
#
**Files:**
- Modify: `src/core/exchange/polymarket/client.rs`
- Modify: `src/app/config/polymarket.rs`
- Test: `tests/exchange_tests.rs`
#
**Step 1: Add failing timeout test**
#
```rust
#[tokio::test]
async fn client_times_out_on_slow_response() { /* ... */ }
```
#
**Step 2: Run test to verify failure**
#
Run: `cargo test tests::exchange_tests::client_times_out_on_slow_response -v`
Expected: FAIL
#
**Step 3: Implement timeout config and retry**
#
```rust
// Add timeout fields to config; apply to reqwest client
```
#
**Step 4: Run test to verify pass**
#
Run: `cargo test tests::exchange_tests::client_times_out_on_slow_response -v`
Expected: PASS
#
**Step 5: Commit**
#
```bash
git add src/core/exchange/polymarket/client.rs src/app/config/polymarket.rs tests/exchange_tests.rs
git commit -m "fix(exchange): add timeouts and retries"
```
#
### Task 2.3: Graceful shutdown and health check
#
**Files:**
- Modify: `src/app/orchestrator/mod.rs`
- Modify: `src/cli/check/mod.rs`
- Test: `tests/health_tests.rs`
#
**Step 1: Add failing health check test**
#
```rust
#[test]
fn health_check_reports_critical_services() { /* ... */ }
```
#
**Step 2: Run test to verify failure**
#
Run: `cargo test tests::health_tests::health_check_reports_critical_services -v`
Expected: FAIL
#
**Step 3: Implement shutdown + health**
#
```rust
// Add shutdown signals and health status reporting
```
#
**Step 4: Run test to verify pass**
#
Run: `cargo test tests::health_tests::health_check_reports_critical_services -v`
Expected: PASS
#
**Step 5: Commit**
#
```bash
git add src/app/orchestrator/mod.rs src/cli/check/mod.rs tests/health_tests.rs
git commit -m "feat(app): add graceful shutdown and health check"
```
#
---
#
## Phase 3: Config + CLI Normalization
#
### Task 3.1: Strict config validation and defaults
#
**Files:**
- Modify: `src/app/config/mod.rs`
- Modify: `src/app/config/*.rs`
- Test: `tests/config_tests.rs`
#
**Step 1: Add failing validation tests**
#
```rust
#[test]
fn config_rejects_invalid_slippage() { /* ... */ }
#[test]
fn config_rejects_missing_exchange_urls() { /* ... */ }
```
#
**Step 2: Run tests to verify failure**
#
Run: `cargo test tests::config_tests::config_rejects_invalid_slippage -v`
Expected: FAIL
#
**Step 3: Implement validation and defaults**
#
```rust
// Enforce ranges, relationships, and required fields
```
#
**Step 4: Run tests to verify pass**
#
Run: `cargo test tests::config_tests::config_rejects_invalid_slippage -v`
Expected: PASS
#
**Step 5: Commit**
#
```bash
git add src/app/config/mod.rs src/app/config/*.rs tests/config_tests.rs
git commit -m "refactor(config): strict validation and defaults"
```
#
### Task 3.2: CLI consistency and error typing
#
**Files:**
- Modify: `src/cli/*.rs`
- Test: `tests/cli_tests.rs`
#
**Step 1: Add failing CLI error tests**
#
```rust
#[test]
fn cli_returns_nonzero_on_config_error() { /* ... */ }
```
#
**Step 2: Run tests to verify failure**
#
Run: `cargo test tests::cli_tests::cli_returns_nonzero_on_config_error -v`
Expected: FAIL
#
**Step 3: Implement consistent error handling**
#
```rust
// Normalize error types and exit codes across commands
```
#
**Step 4: Run tests to verify pass**
#
Run: `cargo test tests::cli_tests::cli_returns_nonzero_on_config_error -v`
Expected: PASS
#
**Step 5: Commit**
#
```bash
git add src/cli/*.rs tests/cli_tests.rs
git commit -m "refactor(cli): normalize error handling"
```
#
---
#
## Verification
#
Run full suite after each phase:
#
```bash
cargo test
```
#
Expected: PASS
#
---
#
## Notes
#
- Breaking changes are acceptable. Remove or rename config/CLI fields without compatibility shims.
- Document the final config schema and operational defaults in `doc/configuration.md`.
- Use typed errors, avoid `unwrap()`/`expect()` in production paths.
