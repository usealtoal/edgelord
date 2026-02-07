# Integration Hardening and Validation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add deterministic end-to-end integration coverage, exchange smoke validation, and CI gates so mainnet-facing changes are safer by default.

**Architecture:** Keep deterministic integration tests separate from live/smoke tests. Deterministic tests run on every PR in CI. Live smoke tests remain `#[ignore]`, manually triggered, and env-gated with dedicated credentials and strict notional caps. Reuse the existing exchange abstraction to avoid exchange-specific test code leaking into core flow tests.

**Tech Stack:** Rust (`tokio`, existing test support modules), GitHub Actions, optional `cargo-llvm-cov`.

### Task 1: Add Shared Integration Harness

**Files:**
- Create: `/Users/rdekovich/workspace/altoal/edgelord/tests/harness/mod.rs`
- Create: `/Users/rdekovich/workspace/altoal/edgelord/tests/harness/scripted_stream.rs`
- Create: `/Users/rdekovich/workspace/altoal/edgelord/tests/harness/recording_notifier.rs`
- Create: `/Users/rdekovich/workspace/altoal/edgelord/tests/harness/temp_db.rs`
- Modify: `/Users/rdekovich/workspace/altoal/edgelord/tests/support/mod.rs`

**Step 1: Write failing harness import test**
- Create `/Users/rdekovich/workspace/altoal/edgelord/tests/e2e_flow_tests.rs` with a basic compile-time import of `tests::harness`.

**Step 2: Run test and confirm failure**
- Run: `cargo test --test e2e_flow_tests -v`
- Expected: unresolved module/symbol errors.

**Step 3: Implement minimal harness modules**
- Add scripted market-data stream, recording notifier collector, and temp SQLite helper.

**Step 4: Re-run test and confirm pass**
- Run: `cargo test --test e2e_flow_tests -v`
- Expected: test binary compiles and test passes.

### Task 2: Add Deterministic End-to-End Flow Test

**Files:**
- Modify: `/Users/rdekovich/workspace/altoal/edgelord/tests/e2e_flow_tests.rs`
- Modify: `/Users/rdekovich/workspace/altoal/edgelord/src/app/orchestrator/builder.rs`
- Modify: `/Users/rdekovich/workspace/altoal/edgelord/src/app/orchestrator/mod.rs`

**Step 1: Write failing test for ingest -> detect -> persist -> notify**
- Assert that a synthetic arbitrage sequence produces one persisted opportunity and one notification.

**Step 2: Run test and confirm failure**
- Run: `cargo test --test e2e_flow_tests e2e_ingest_detect_persist_notify -v`
- Expected: failure due to missing injection/test hook.

**Step 3: Add minimal dependency injection seam**
- Add constructor/path that accepts test doubles for stream/notifier/storage path.

**Step 4: Re-run focused test**
- Run: `cargo test --test e2e_flow_tests e2e_ingest_detect_persist_notify -v`
- Expected: pass.

### Task 3: Expand Provisioning/Wallet Integration Matrix

**Files:**
- Modify: `/Users/rdekovich/workspace/altoal/edgelord/tests/cli_provision_tests.rs`
- Modify: `/Users/rdekovich/workspace/altoal/edgelord/tests/cli_wallet_tests.rs`

**Step 1: Add failing tests for edge cases**
- Keystore exists already.
- Missing `EDGELORD_PRIVATE_KEY` on import mode.
- Sweep with unsupported asset/network.

**Step 2: Run targeted tests and confirm failures**
- Run: `cargo test --test cli_provision_tests --test cli_wallet_tests -v`

**Step 3: Fix behavior or assertions**
- Keep behavior strict; normalize errors where needed.

**Step 4: Re-run targeted tests**
- Run: `cargo test --test cli_provision_tests --test cli_wallet_tests -v`
- Expected: all pass.

### Task 4: Add Live Smoke Suite (Manual, Env-Gated)

**Files:**
- Create: `/Users/rdekovich/workspace/altoal/edgelord/tests/polymarket_live_smoke_tests.rs`
- Create: `/Users/rdekovich/workspace/altoal/edgelord/docs/deployment/mainnet-validation.md`

**Step 1: Add ignored smoke tests**
- `#[ignore]` tests for: connectivity, wallet address derivation, approval status read.
- Require envs like `EDGELORD_SMOKE=1`, `EDGELORD_SMOKE_ALLOW_SPEND=0`.

**Step 2: Run locally without envs**
- Run: `cargo test --test polymarket_live_smoke_tests -v`
- Expected: skipped/guarded behavior.

**Step 3: Document safe execution**
- Add exact command/env examples and risk guardrails in docs.

### Task 5: Add CI Coverage and Integration Gates

**Files:**
- Modify: `/Users/rdekovich/workspace/altoal/edgelord/.github/workflows/ci.yml`
- Create: `/Users/rdekovich/workspace/altoal/edgelord/.github/workflows/smoke.yml`
- Modify: `/Users/rdekovich/workspace/altoal/edgelord/Cargo.toml`

**Step 1: Add deterministic integration job**
- Explicitly run integration suites (`cargo test --tests`) in CI.

**Step 2: Add coverage gate**
- Add `cargo llvm-cov` step with fail-under threshold (start at 60%-70% and raise over time).

**Step 3: Add manual smoke workflow**
- `workflow_dispatch` with protected environment and required secrets.

**Step 4: Validate workflows**
- Run: `cargo test`
- Run YAML lint (if available) or CI dry run on branch.

### Task 6: Add Deployment Validation Hooks

**Files:**
- Modify: `/Users/rdekovich/workspace/altoal/edgelord/.github/workflows/deploy.yml`

**Step 1: Add pre-restart validation command on host**
- `edgelord check config --config ...`
- `edgelord check live --config ...` in strict mode.

**Step 2: Add rollback trigger on validation fail**
- Keep existing backup restore path; fail deployment before traffic resumes.

### Task 7: Documentation and Runbook

**Files:**
- Create: `/Users/rdekovich/workspace/altoal/edgelord/docs/testing/integration-matrix.md`
- Modify: `/Users/rdekovich/workspace/altoal/edgelord/README.md`

**Step 1: Document test tiers**
- Unit vs deterministic integration vs live smoke.

**Step 2: Document local commands**
- One command block per tier and expected runtime.

**Step 3: Document credentials and safe limits**
- Explicitly list minimal required secrets and max allowed notional for smoke.

### Task 8: Verification and Merge

**Files:**
- Modify: `/Users/rdekovich/workspace/altoal/edgelord/docs/plans/2026-02-07-integration-hardening-and-validation.md`

**Step 1: Run full validation**
- `cargo test`
- `cargo check`
- CI workflow pass on branch.

**Step 2: Record outcomes**
- Update this plan with final status notes and any deferred items.

**Step 3: Commit in small units**
- One logical commit per task group.
