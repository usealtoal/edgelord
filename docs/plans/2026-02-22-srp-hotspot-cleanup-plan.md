# SRP Hotspot Cleanup Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Improve SRP and readability in the three remaining hotspots without changing behavior or weakening hexagonal boundaries.

**Architecture:** Keep public module paths stable and split internals into focused submodules (façade + cohesive units). Use characterization tests first, then refactor in small slices. Preserve all existing architecture contracts and add new ones where needed.

**Tech Stack:** Rust, cargo test, cargo clippy, architecture contracts (`tests/architecture_contract_tests.rs`), architecture script (`tools/check-architecture.sh`).

### Task 1: Lock Behavior with Characterization Tests

**Files:**
- Modify: `tests/architecture_contract_tests.rs`
- Modify: `tools/check-architecture.sh`
- Test: `tests/architecture_contract_tests.rs`

**Step 1: Write the failing tests/contracts**

- Add/extend contracts for:
  - Pool module split shape.
  - Telegram control split shape.
  - Priority manager split shape.
  - Façade-thin expectations for each old hotspot entrypoint.

**Step 2: Run test to verify it fails**

Run: `cargo test --test architecture_contract_tests -q`  
Expected: FAIL on new split/façade assertions.

**Step 3: Write minimal implementation**

- Only add the minimum contracts and script checks required to drive refactor.

**Step 4: Run test to verify it passes**

Run: `cargo test --test architecture_contract_tests -q`  
Expected: PASS for existing + new contract checks.

**Step 5: Commit**

```bash
git add tests/architecture_contract_tests.rs tools/check-architecture.sh
git commit -m "test(architecture): add SRP hotspot split contracts"
```

### Task 2: Split Connection Pool Hotspot

**Files:**
- Create: `src/infrastructure/exchange/pool/mod.rs`
- Create: `src/infrastructure/exchange/pool/state.rs`
- Create: `src/infrastructure/exchange/pool/spawn.rs`
- Create: `src/infrastructure/exchange/pool/replace.rs`
- Create: `src/infrastructure/exchange/pool/manage.rs`
- Create: `src/infrastructure/exchange/pool/pool.rs`
- Modify: `src/infrastructure/exchange/mod.rs`
- Delete or shrink façade: `src/infrastructure/exchange/pool.rs` (module path preserved)
- Test: existing tests in `src/infrastructure/exchange/pool.rs` (moved alongside split)

**Step 1: Write the failing test**

- Keep the Task 1 architecture checks red until split files exist and façades are thin.

**Step 2: Run test to verify it fails**

Run: `cargo test --test architecture_contract_tests -q`  
Expected: FAIL on pool split assertions before extraction.

**Step 3: Write minimal implementation**

- Move types/state helpers to `state.rs`.
- Move connection spawn/build to `spawn.rs`.
- Move handoff/rotation logic to `replace.rs`.
- Move management loop to `manage.rs`.
- Keep `pool.rs` responsible for `ConnectionPool` API and `MarketDataStream` impl.
- Keep `mod.rs` export-only.

**Step 4: Run test to verify it passes**

Run: `cargo test -q -- infrastructure::exchange::pool`  
Expected: PASS with behavior unchanged.

**Step 5: Commit**

```bash
git add src/infrastructure/exchange/pool src/infrastructure/exchange/mod.rs
git commit -m "refactor(pool): split connection pool into focused modules"
```

### Task 3: Split Telegram Control Hotspot

**Files:**
- Create: `src/adapter/outbound/notifier/telegram/control/mod.rs`
- Create: `src/adapter/outbound/notifier/telegram/control/runtime.rs`
- Create: `src/adapter/outbound/notifier/telegram/control/dispatch.rs`
- Create: `src/adapter/outbound/notifier/telegram/control/render.rs`
- Create: `src/adapter/outbound/notifier/telegram/control/mutate.rs`
- Modify: `src/adapter/outbound/notifier/telegram/mod.rs`
- Delete or shrink façade: `src/adapter/outbound/notifier/telegram/control.rs`
- Test: existing tests currently in `src/adapter/outbound/notifier/telegram/control.rs` (moved)

**Step 1: Write the failing test**

- Enforce split shape via architecture contracts from Task 1.

**Step 2: Run test to verify it fails**

Run: `cargo test --test architecture_contract_tests -q`  
Expected: FAIL on telegram split assertions before extraction.

**Step 3: Write minimal implementation**

- `runtime.rs`: `RuntimeStats`.
- `dispatch.rs`: command routing entrypoint.
- `render.rs`: read-only response builders.
- `mutate.rs`: risk/circuit-breaker state changes.
- Keep module path and public types stable.

**Step 4: Run test to verify it passes**

Run: `cargo test -q -- telegram::control`  
Expected: PASS with identical command behavior/output.

**Step 5: Commit**

```bash
git add src/adapter/outbound/notifier/telegram
git commit -m "refactor(telegram): split control command runtime and rendering"
```

### Task 4: Split Priority Subscription Hotspot

**Files:**
- Create: `src/infrastructure/subscription/priority/mod.rs`
- Create: `src/infrastructure/subscription/priority/state.rs`
- Create: `src/infrastructure/subscription/priority/queue.rs`
- Create: `src/infrastructure/subscription/priority/contract.rs`
- Create: `src/infrastructure/subscription/priority/event.rs`
- Modify: `src/infrastructure/subscription/mod.rs`
- Delete or shrink façade: `src/infrastructure/subscription/priority.rs`
- Test: existing tests currently in `src/infrastructure/subscription/priority.rs` (moved)

**Step 1: Write the failing test**

- Use Task 1 contract checks for split shape/façade thinness.

**Step 2: Run test to verify it fails**

Run: `cargo test --test architecture_contract_tests -q`  
Expected: FAIL on priority split assertions before extraction.

**Step 3: Write minimal implementation**

- `state.rs`: lock wrappers + state containers.
- `queue.rs`: enqueue/expand logic.
- `contract.rs`: contract/downscale logic.
- `event.rs`: connection event handling.
- Keep trait impl path stable.

**Step 4: Run test to verify it passes**

Run: `cargo test -q -- subscription::priority`  
Expected: PASS with unchanged queue behavior.

**Step 5: Commit**

```bash
git add src/infrastructure/subscription
git commit -m "refactor(subscription): split priority manager responsibilities"
```

### Task 5: Final Verification and Purity Gate

**Files:**
- Modify: `tools/check-architecture.sh`
- Modify: `tests/architecture_contract_tests.rs`

**Step 1: Run full verification**

Run:

```bash
cargo fmt --check
cargo check --all-targets
cargo test -q
cargo clippy --all-targets -- -D warnings
bash tools/check-architecture.sh
```

Expected: all commands pass.

**Step 2: Commit**

```bash
git add tests/architecture_contract_tests.rs tools/check-architecture.sh
git commit -m "chore(architecture): enforce SRP splits for remaining hotspots"
```
