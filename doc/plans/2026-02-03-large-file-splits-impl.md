# Large File Splits Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Split 8 files exceeding the 400-line limit to comply with ARCHITECTURE.md standards.

**Architecture:** Test extraction for most files, responsibility separation for orchestrator, type extraction for score module. All splits preserve public API through re-exports.

**Tech Stack:** Rust, cargo test

**Commit Format:** One line, no co-authorship

---

## Phase 1: Test Extractions

### Task 1.1: Extract tests from subscription/priority.rs

**Files:**
- Modify: `src/core/service/subscription/priority.rs`
- Create: `src/core/service/subscription/tests.rs`
- Modify: `src/core/service/subscription/mod.rs`

**Step 1: Create tests.rs with test module**

Extract the entire `#[cfg(test)] mod tests { ... }` block (lines ~313-753) from priority.rs to a new file `tests.rs`.

The tests.rs file should start with:
```rust
//! Tests for PrioritySubscriptionManager.

use super::*;
```

Then include all the test code.

**Step 2: Remove tests from priority.rs**

Delete the `#[cfg(test)] mod tests { ... }` block from priority.rs.

**Step 3: Update subscription/mod.rs**

Add:
```rust
#[cfg(test)]
mod tests;
```

**Step 4: Build and test**

```bash
cargo build && cargo test subscription
```

**Step 5: Commit**

```bash
git add -A && git commit -m "refactor(subscription): extract priority tests to separate file"
```

---

### Task 1.2: Extract tests from governor/latency.rs

**Files:**
- Modify: `src/core/service/governor/latency.rs`
- Create: `src/core/service/governor/latency_tests.rs`
- Modify: `src/core/service/governor/mod.rs`

**Step 1: Create latency_tests.rs**

Extract the `#[cfg(test)] mod tests { ... }` block (~lines 229-629) from latency.rs.

```rust
//! Tests for LatencyGovernor.

use super::*;
use super::super::{LatencyTargets, ScalingConfig};
```

**Step 2: Remove tests from latency.rs**

Delete the test module from latency.rs.

**Step 3: Update governor/mod.rs**

Add after the latency module import:
```rust
#[cfg(test)]
mod latency_tests;
```

**Step 4: Build and test**

```bash
cargo build && cargo test governor
```

**Step 5: Commit**

```bash
git add -A && git commit -m "refactor(governor): extract latency tests to separate file"
```

---

### Task 1.3: Extract tests from governor/mod.rs

**Files:**
- Modify: `src/core/service/governor/mod.rs`
- Create: `src/core/service/governor/tests.rs`

**Step 1: Create tests.rs**

Extract the `#[cfg(test)] mod tests { ... }` block (~lines 301-428) from mod.rs.

```rust
//! Tests for governor configuration and traits.

use super::*;
```

**Step 2: Remove tests from mod.rs**

Delete the test module.

**Step 3: Add to mod.rs**

```rust
#[cfg(test)]
mod tests;
```

**Step 4: Build and test**

```bash
cargo build && cargo test governor
```

**Step 5: Commit**

```bash
git add -A && git commit -m "refactor(governor): extract config tests to separate file"
```

---

### Task 1.4: Extract tests from condition/single.rs

**Files:**
- Modify: `src/core/strategy/condition/single.rs`
- Create: `src/core/strategy/condition/tests.rs`
- Modify: `src/core/strategy/condition/mod.rs`

**Step 1: Create tests.rs**

Extract tests (~lines 151-423) from single.rs.

```rust
//! Tests for SingleConditionStrategy.

use super::*;
use super::super::super::{DetectionContext, MarketContext, Strategy};
```

**Step 2: Remove tests from single.rs**

**Step 3: Update condition/mod.rs**

Add:
```rust
#[cfg(test)]
mod tests;
```

**Step 4: Build and test**

```bash
cargo build && cargo test single_condition
```

**Step 5: Commit**

```bash
git add -A && git commit -m "refactor(strategy): extract condition tests to separate file"
```

---

### Task 1.5: Extract tests from rebalancing/mod.rs

**Files:**
- Modify: `src/core/strategy/rebalancing/mod.rs`
- Create: `src/core/strategy/rebalancing/tests.rs`

**Step 1: Create tests.rs**

Extract tests (~lines 256-556) from mod.rs.

```rust
//! Tests for MarketRebalancingStrategy.

use super::*;
```

**Step 2: Remove tests from mod.rs**

**Step 3: Add to mod.rs**

```rust
#[cfg(test)]
mod tests;
```

**Step 4: Build and test**

```bash
cargo build && cargo test rebalancing
```

**Step 5: Commit**

```bash
git add -A && git commit -m "refactor(strategy): extract rebalancing tests to separate file"
```

---

### Task 1.6: Extract tests from domain/score.rs

**Files:**
- Modify: `src/core/domain/score.rs`
- Create: `src/core/domain/score_tests.rs`
- Modify: `src/core/domain/mod.rs`

**Step 1: Create score_tests.rs**

Extract tests (~lines 232-416) from score.rs.

```rust
//! Tests for market scoring types.

use super::*;
```

Note: The tests reference types directly, so `use super::*` should work, but you may need to import `MarketId` from the parent module.

**Step 2: Remove tests from score.rs**

**Step 3: Update domain/mod.rs**

Add:
```rust
#[cfg(test)]
mod score_tests;
```

**Step 4: Build and test**

```bash
cargo build && cargo test score
```

**Step 5: Commit**

```bash
git add -A && git commit -m "refactor(domain): extract score tests to separate file"
```

---

### Task 1.7: Extract tests from app/status.rs

**Files:**
- Modify: `src/app/status.rs`
- Create: `src/app/status_tests.rs`
- Modify: `src/app/mod.rs`

**Step 1: Create status_tests.rs**

Extract tests (~lines 176-406) from status.rs.

```rust
//! Tests for status file writing.

use super::*;
use tempfile::TempDir;
```

**Step 2: Remove tests from status.rs**

**Step 3: Update app/mod.rs**

Add:
```rust
#[cfg(test)]
mod status_tests;
```

**Step 4: Build and test**

```bash
cargo build && cargo test status
```

**Step 5: Commit**

```bash
git add -A && git commit -m "refactor(app): extract status tests to separate file"
```

---

## Phase 2: Orchestrator Split

### Task 2.1: Create orchestrator module structure

**Files:**
- Create: `src/app/orchestrator/mod.rs`
- Create: `src/app/orchestrator/builder.rs`
- Create: `src/app/orchestrator/handler.rs`
- Create: `src/app/orchestrator/execution.rs`
- Delete: `src/app/orchestrator.rs`
- Modify: `src/app/mod.rs`

**Step 1: Create orchestrator directory**

```bash
mkdir -p src/app/orchestrator
```

**Step 2: Read current orchestrator.rs and split**

Read the file and split into:

**builder.rs** (~110 lines):
- `build_notifier_registry()` function
- `build_strategy_registry()` function
- `init_executor()` function

**handler.rs** (~150 lines):
- `handle_market_event()` function
- `handle_opportunity()` function
- `get_max_slippage()` function

**execution.rs** (~120 lines):
- `spawn_execution()` function
- `record_position()` function
- `record_partial_position()` function

**mod.rs** (~180 lines):
- Imports from submodules
- `App` struct
- `run()` method (calls into submodule functions)
- Re-exports

**Step 3: Move orchestrator.rs to orchestrator/mod.rs**

```bash
mv src/app/orchestrator.rs src/app/orchestrator/mod.rs
```

**Step 4: Extract builder.rs**

Create builder.rs with the builder functions. Update mod.rs to:
- Add `mod builder;`
- Use `builder::build_notifier_registry()` etc.

**Step 5: Extract handler.rs**

Create handler.rs with handler functions. Update mod.rs.

**Step 6: Extract execution.rs**

Create execution.rs with execution functions. Update mod.rs.

**Step 7: Build and test**

```bash
cargo build && cargo test
```

**Step 8: Commit**

```bash
git add -A && git commit -m "refactor(app): split orchestrator into submodules"
```

---

## Phase 3: Final Verification

### Task 3.1: Verify all files under 400 lines

**Step 1: Check line counts**

```bash
find src -name "*.rs" -exec wc -l {} \; | sort -rn | head -20
```

Expected: All files under 400 lines.

**Step 2: Run full test suite**

```bash
cargo test
```

**Step 3: Run clippy**

```bash
cargo clippy -- -D warnings
```

**Step 4: Fix any issues and commit**

```bash
git add -A && git commit -m "chore: final cleanup after file splits"
```

---

## Summary

| Phase | Tasks | Files Split |
|-------|-------|-------------|
| 1 | 1.1-1.7 | 7 test extractions |
| 2 | 2.1 | orchestrator → 4 files |
| 3 | 3.1 | Verification |

**Total tasks:** 9
**Expected result:** All source files under 400 lines
