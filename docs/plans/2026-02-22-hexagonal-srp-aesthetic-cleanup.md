# Hexagonal SRP Aesthetic Cleanup Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make the architecture consistently clean and SRP-driven by codifying naming, boundary, and module-shape rules and enforcing them in tests/CI.

**Architecture:** Keep the current hexagonal shape (`domain`, `application`, `port`, `adapter`, `infrastructure`) and improve cleanliness through guardrails, not new layers. Split responsibilities by capability (already started in operator/exchange) and remove residual naming drift. Add architecture contract tests so future changes cannot silently regress boundaries or module hygiene.

**Tech Stack:** Rust, Cargo, integration tests, ripgrep, GitHub Actions.

### Task 1: Add Architecture Contract Tests (Boundary + Naming)

**Files:**
- Create: `tests/architecture_contract_tests.rs`
- Create: `tests/support/architecture.rs`
- Modify: `tests/support/mod.rs`

**Step 1: Write the failing test**

Create strict tests first:

```rust
#[test]
fn cli_has_no_infrastructure_imports() {
    // intentionally strict first pass (will fail because of cli/operator bridge)
}

#[test]
fn domain_has_no_infrastructure_framework_imports() {
    // check domain does not import adapter/infrastructure/tokio/reqwest/sqlx
}

#[test]
fn legacy_exchange_config_port_is_removed() {
    // assert src/port/outbound/exchange_config.rs does not exist
}
```

**Step 2: Run test to verify it fails**

Run:

```bash
cargo test architecture_contract_tests -q
```

Expected: FAIL on `cli_has_no_infrastructure_imports` (bridge exception not yet modeled).

**Step 3: Write minimal implementation**

Add `tests/support/architecture.rs` helpers:
- File scanning helper with `walkdir` or std recursive traversal.
- `forbidden_use_lines(root, forbidden_patterns)` utility.
- `mod_rs_non_export_lines(root)` utility.

Update tests to allow exactly one exception path:
- `src/adapter/inbound/cli/operator.rs`

**Step 4: Run test to verify it passes**

Run:

```bash
cargo test architecture_contract_tests -q
```

Expected: PASS.

**Step 5: Commit**

```bash
git add tests/architecture_contract_tests.rs tests/support/architecture.rs tests/support/mod.rs
git commit -m "test(architecture): enforce hexagonal boundary contracts"
```

### Task 2: Enforce `mod.rs` Export-Only Hygiene in Tests

**Files:**
- Modify: `tests/architecture_contract_tests.rs`
- Modify: `tests/support/architecture.rs`

**Step 1: Write the failing test**

Add a failing test that checks every `mod.rs` only contains:
- comments
- `pub mod ...;`
- `mod ...;`
- `#[cfg(...)]`

No other code/attributes/items.

**Step 2: Run test to verify it fails**

Run:

```bash
cargo test architecture_contract_tests::mod_rs_is_export_only -q
```

Expected: FAIL if any module slips non-export content.

**Step 3: Write minimal implementation**

Add parser helper in `tests/support/architecture.rs`:
- line-level matcher for allowed patterns.
- clear failure output with path + line.

**Step 4: Run test to verify it passes**

Run:

```bash
cargo test architecture_contract_tests::mod_rs_is_export_only -q
```

Expected: PASS.

**Step 5: Commit**

```bash
git add tests/architecture_contract_tests.rs tests/support/architecture.rs
git commit -m "test(mod): enforce export-only mod.rs files"
```

### Task 3: Normalize Operator Bridge Naming for Clarity

**Files:**
- Modify: `src/adapter/inbound/cli/operator.rs`
- Modify: `src/adapter/inbound/cli/config.rs`
- Modify: `src/adapter/inbound/cli/run.rs`
- Modify: `src/adapter/inbound/cli/status.rs`
- Modify: `src/adapter/inbound/cli/stats.rs`
- Modify: `src/adapter/inbound/cli/check/config.rs`
- Modify: `src/adapter/inbound/cli/check/live.rs`
- Modify: `src/adapter/inbound/cli/check/connection.rs`
- Modify: `src/adapter/inbound/cli/check/health.rs`
- Modify: `src/adapter/inbound/cli/check/telegram.rs`
- Modify: `src/adapter/inbound/cli/wallet/address.rs`
- Modify: `src/adapter/inbound/cli/wallet/status.rs`
- Modify: `src/adapter/inbound/cli/wallet/approve.rs`
- Modify: `src/adapter/inbound/cli/wallet/sweep.rs`

**Step 1: Write the failing test**

Add/extend architecture tests:
- `operator_bridge_fn_name_is_operator` (if decision is to use `operator()` instead of `bridge()`).

**Step 2: Run test to verify it fails**

Run:

```bash
cargo test architecture_contract_tests::operator_bridge_fn_name_is_operator -q
```

Expected: FAIL while function remains `bridge`.

**Step 3: Write minimal implementation**

Rename function:
- `bridge()` -> `operator()`

Update all CLI call sites accordingly.

**Step 4: Run test to verify it passes**

Run:

```bash
cargo test architecture_contract_tests::operator_bridge_fn_name_is_operator -q
cargo check -q
```

Expected: PASS.

**Step 5: Commit**

```bash
git add src/adapter/inbound/cli
 git commit -m "refactor(cli): use consistent operator bridge naming"
```

### Task 4: Add Architecture Guard Script and Wire CI

**Files:**
- Create: `tools/check-architecture.sh`
- Modify: `.github/workflows/ci.yml`

**Step 1: Write the failing check command**

Create script with strict checks:
- no `exchange_config` references
- no monolithic operator file
- no direct infrastructure imports from `src/adapter/inbound/cli` except `cli/operator.rs`
- optional: fail if `mod.rs` has non-export lines (reuse grep/awk approach)

Initially add one intentionally strict rule that fails (`no infrastructure imports at all`).

**Step 2: Run check to verify it fails**

Run:

```bash
bash tools/check-architecture.sh
```

Expected: FAIL due to known bridge exception.

**Step 3: Write minimal implementation**

Adjust script with explicit allowlist for `src/adapter/inbound/cli/operator.rs`.

Wire into CI `check` job:

```yaml
- name: Check architecture contracts
  run: bash tools/check-architecture.sh
```

**Step 4: Run check to verify it passes**

Run:

```bash
bash tools/check-architecture.sh
cargo check -q
```

Expected: PASS.

**Step 5: Commit**

```bash
git add tools/check-architecture.sh .github/workflows/ci.yml
git commit -m "ci(architecture): enforce boundary and naming checks"
```

### Task 5: Documentation Aesthetic Pass (Single Vocabulary)

**Files:**
- Modify: `ARCHITECTURE.md`
- Modify: `docs/architecture/overview.md`
- Modify: `docs/extending/architecture.md`
- Modify: `docs/extending/exchanges.md`
- Modify: `README.md`

**Step 1: Write the failing doc consistency check**

Add temporary grep check commands to detect old terms:
- `ExchangeConfig`
- `exchange_config` (as port concept)
- `market_mapper`
- `operator service` when referring to current split modules

**Step 2: Run check to verify it fails**

Run:

```bash
rg "ExchangeConfig|exchange_config|market_mapper|operator service" ARCHITECTURE.md docs README.md
```

Expected: FAIL (matches found).

**Step 3: Write minimal implementation**

Update docs to use canonical vocabulary:
- `MarketParser`
- `operator/{configuration,diagnostic,runtime,statistics,status,wallet}`
- Bridge rule explicitly documented.

**Step 4: Run check to verify it passes**

Run:

```bash
rg "ExchangeConfig|market_mapper" ARCHITECTURE.md docs README.md
cargo test --doc -q
```

Expected: no matches for deprecated terms; doc tests pass.

**Step 5: Commit**

```bash
git add ARCHITECTURE.md docs README.md
git commit -m "docs(architecture): align terminology and module map"
```

### Task 6: Full Verification + Final Sweep

**Files:**
- Modify: (none expected; only if failures found)

**Step 1: Run full verification matrix**

Run:

```bash
cargo fmt --check
cargo check --all-targets
cargo test -q
cargo clippy --all-targets -- -D warnings
bash tools/check-architecture.sh
```

Expected: all PASS.

**Step 2: If anything fails, fix only that scope**

- Apply smallest possible patch.
- Re-run only failing command, then full matrix.

**Step 3: Generate final cleanup report**

Capture:
- enforced architecture invariants
- renamed/removed concepts
- known intentional exceptions (CLI bridge)

**Step 4: Commit final sweep**

```bash
git add -A
git commit -m "chore(architecture): complete SRP and aesthetic cleanup guardrails"
```

**Step 5: Optional PR checklist**

- include before/after tree snippets
- include boundary-test output
- include CI job references

## Execution Notes

- Follow `@superpowers:test-driven-development` for each task.
- For broad task execution across this plan, use `@superpowers:executing-plans`.
- Keep commits small and thematic (one task per commit when possible).
- Do not add new layers; enforce cleanliness through naming + guardrails.
