# Singular File Names Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rename plural module files (`types.rs`, `traits.rs`) to singular, descriptive names and update references across the codebase.

**Architecture:** Keep module boundaries intact; only rename files and module identifiers to singular equivalents. Update naming rules in `ARCHITECTURE.md` to reflect the new convention.

**Tech Stack:** Rust 2021, Cargo.

### Task 1: Update Naming Convention Documentation

**Files:**
- Modify: `ARCHITECTURE.md`

**Step 1: Update naming exceptions**

Replace the current exception block with a singular naming rule that avoids generic plurals:
```markdown
### Exceptions

- Avoid generic plurals like `types.rs` or `traits.rs`. Use descriptive singular names instead (e.g., `exchange_config.rs`, `response.rs`, `stat.rs`).
- `statistics/` is allowed as a singular service name for aggregated stats.
```

**Step 2: Verification**

No tests required (doc-only change).

**Step 3: Commit**

```bash
git add ARCHITECTURE.md
git commit -m "docs(architecture): prefer singular file names"
```

### Task 2: Rename Exchange Config Module (traits → exchange_config)

**Files:**
- Move: `src/core/exchange/traits.rs` → `src/core/exchange/exchange_config.rs`
- Modify: `src/core/exchange/mod.rs`
- Modify: any imports using `core::exchange::traits`

**Step 1: Rename file**

```bash
git mv src/core/exchange/traits.rs src/core/exchange/exchange_config.rs
```

**Step 2: Update module wiring**

In `src/core/exchange/mod.rs`, replace:
```rust
mod traits;
pub use traits::ExchangeConfig;
```
With:
```rust
mod exchange_config;
pub use exchange_config::ExchangeConfig;
```

**Step 3: Update imports**

Replace any `use crate::core::exchange::traits::...` or `core::exchange::traits::...` with `exchange_config`.

**Step 4: Run tests**

Run:
```bash
cargo test --test exchange_tests
```
Expected: PASS.

**Step 5: Commit**

```bash
git add src/core/exchange/exchange_config.rs src/core/exchange/mod.rs
git commit -m "refactor(exchange): rename traits module"
```

### Task 3: Rename Polymarket Response Module (types → response)

**Files:**
- Move: `src/core/exchange/polymarket/types.rs` → `src/core/exchange/polymarket/response.rs`
- Modify: `src/core/exchange/polymarket/mod.rs`
- Modify: any imports using `polymarket::types`

**Step 1: Rename file**

```bash
git mv src/core/exchange/polymarket/types.rs src/core/exchange/polymarket/response.rs
```

**Step 2: Update module wiring**

In `src/core/exchange/polymarket/mod.rs`, replace:
```rust
mod types;
```
With:
```rust
mod response;
```

**Step 3: Update imports**

Replace any `use crate::core::exchange::polymarket::types::...` with `response`.

**Step 4: Run tests**

Run:
```bash
cargo test --test exchange_tests
```
Expected: PASS.

**Step 5: Commit**

```bash
git add src/core/exchange/polymarket/response.rs src/core/exchange/polymarket/mod.rs
git commit -m "refactor(polymarket): rename types module"
```

### Task 4: Rename Statistics Types Module (types → stat)

**Files:**
- Move: `src/core/service/statistics/types.rs` → `src/core/service/statistics/stat.rs`
- Modify: `src/core/service/statistics/mod.rs`
- Modify: any imports using `statistics::types`

**Step 1: Rename file**

```bash
git mv src/core/service/statistics/types.rs src/core/service/statistics/stat.rs
```

**Step 2: Update module wiring**

In `src/core/service/statistics/mod.rs`, replace:
```rust
mod types;
pub use types::{
    OpportunitySummary, RecordedOpportunity, StatsSummary, TradeCloseEvent, TradeLeg, TradeOpenEvent,
};
```
With:
```rust
mod stat;
pub use stat::{
    OpportunitySummary, RecordedOpportunity, StatsSummary, TradeCloseEvent, TradeLeg, TradeOpenEvent,
};
```

**Step 3: Update imports**

Replace any `use crate::core::service::statistics::types::...` with `stat`.

**Step 4: Run tests**

Run:
```bash
cargo test --test stats_tests
```
Expected: PASS.

**Step 5: Commit**

```bash
git add src/core/service/statistics/stat.rs src/core/service/statistics/mod.rs
git commit -m "refactor(statistics): rename types module"
```

---

Plan complete and saved to `docs/plans/2026-02-06-singular-file-names.md`. Two execution options:

1. Subagent-Driven (this session) - I dispatch fresh subagent per task, review between tasks, fast iteration
2. Parallel Session (separate) - Open new session with executing-plans, batch execution with checkpoints

Which approach?
