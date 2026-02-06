# Provisioning CLI Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add an exchange-specific provisioning CLI (Polymarket-first) with secure headless keystore handling and a `check live` readiness command.

**Architecture:** Introduce a `provision` command group, a Polymarket provisioner module, and shared helpers for keystore creation/import plus config updates. Extend `check` with an exchange-aware `live` command. Keep changes localized to CLI + config wiring.

**Tech Stack:** Rust, clap, toml, existing config loader, Ethereum keystore (via existing crypto deps).

---

### Task 1: Add `provision` command wiring

**Files:**
- Modify: `src/cli/mod.rs`
- Create: `src/cli/provision/mod.rs`
- Modify: `src/cli/run.rs` (if needed for common helpers)

**Step 1: Write the failing CLI test**

```rust
#[test]
fn provision_command_is_registered() {
    // Parse `edgelord provision polymarket` and assert subcommand matches.
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test provision_command_is_registered -v`
Expected: FAIL (unknown subcommand).

**Step 3: Implement minimal CLI wiring**

```rust
pub mod provision;

#[derive(Subcommand, Debug)]
pub enum Commands {
    // ...
    #[command(subcommand)]
    Provision(ProvisionCommand),
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test provision_command_is_registered -v`
Expected: PASS.

**Step 5: Commit**

```bash
git add src/cli/mod.rs src/cli/provision/mod.rs
git commit -m "feat(cli): add provision command scaffold"
```

---

### Task 2: Implement Polymarket provisioning flow (keystore + config update)

**Files:**
- Create: `src/cli/provision/polymarket.rs`
- Modify: `src/cli/provision/mod.rs`
- Modify: `src/app/config/polymarket.rs` (add optional keystore path if needed)
- Modify: `src/app/config/service.rs` or `src/app/config/mod.rs` (wire new field)
- Test: `tests/cli_provision_tests.rs`

**Step 1: Write failing tests for provision output**

```rust
#[test]
fn provision_polymarket_writes_keystore_and_updates_config() {
    // Arrange temp dir + config path
    // Act: run provision with wallet generate + passphrase env
    // Assert: keystore exists + config updated with keystore path
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test provision_polymarket_writes_keystore_and_updates_config -v`
Expected: FAIL (command not implemented).

**Step 3: Implement minimal keystore generation + config update**

```rust
// Read passphrase from env
// Generate wallet keypair
// Write V3 keystore JSON
// Update config with keystore path + exchange="polymarket"
```

**Step 4: Run test to verify it passes**

Run: `cargo test provision_polymarket_writes_keystore_and_updates_config -v`
Expected: PASS.

**Step 5: Commit**

```bash
git add src/cli/provision src/app/config tests/cli_provision_tests.rs
git commit -m "feat(provision): add polymarket keystore setup"
```

---

### Task 3: Add wallet import mode and headless secret handling

**Files:**
- Modify: `src/cli/provision/polymarket.rs`
- Test: `tests/cli_provision_tests.rs`

**Step 1: Write failing test for import mode**

```rust
#[test]
fn provision_polymarket_imports_private_key() {
    // Provide EDGELORD_PRIVATE_KEY + passphrase env
    // Assert keystore is created and address matches
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test provision_polymarket_imports_private_key -v`
Expected: FAIL.

**Step 3: Implement import mode**

```rust
// Read EDGELORD_PRIVATE_KEY and import into keystore
```

**Step 4: Run test to verify it passes**

Run: `cargo test provision_polymarket_imports_private_key -v`
Expected: PASS.

**Step 5: Commit**

```bash
git add src/cli/provision/polymarket.rs tests/cli_provision_tests.rs
git commit -m "feat(provision): support wallet import"
```

---

### Task 4: Add `check live` readiness command

**Files:**
- Modify: `src/cli/check/mod.rs`
- Test: `tests/cli_check_live_tests.rs`

**Step 1: Write failing test for `check live`**

```rust
#[test]
fn check_live_warns_on_missing_wallet_or_mainnet() {
    // Ensure missing wallet or mainnet config produces warnings
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test check_live_warns_on_missing_wallet_or_mainnet -v`
Expected: FAIL.

**Step 3: Implement `check live` logic**

```rust
// If exchange=polymarket: require mainnet chain_id, wallet configured, dry_run=false
```

**Step 4: Run test to verify it passes**

Run: `cargo test check_live_warns_on_missing_wallet_or_mainnet -v`
Expected: PASS.

**Step 5: Commit**

```bash
git add src/cli/check/mod.rs tests/cli_check_live_tests.rs
git commit -m "feat(check): add live readiness check"
```

---

### Task 5: Add wallet address + sweep commands (Polymarket-only)

**Files:**
- Modify: `src/cli/wallet/mod.rs`
- Modify: `src/cli/mod.rs`
- Test: `tests/cli_wallet_tests.rs`

**Step 1: Write failing test for `wallet address`**

```rust
#[test]
fn wallet_address_reads_from_keystore() {
    // Provision keystore then assert address output
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test wallet_address_reads_from_keystore -v`
Expected: FAIL.

**Step 3: Implement address/sweep commands**

```rust
// Load keystore + decrypt -> address
// Sweep uses existing polymarket executor to send USDC
```

**Step 4: Run test to verify it passes**

Run: `cargo test wallet_address_reads_from_keystore -v`
Expected: PASS.

**Step 5: Commit**

```bash
git add src/cli/wallet src/cli/mod.rs tests/cli_wallet_tests.rs
git commit -m "feat(wallet): add address and sweep"
```

---

### Task 6: Documentation updates

**Files:**
- Modify: `README.md`
- Modify: `docs/getting-started.md`
- Modify: `docs/deployment/` (if needed)

**Step 1: Update docs with provisioning flow**

```md
## Provisioning (Polymarket)
- edgelord provision polymarket ...
```

**Step 2: Run docs lint (if any)**

Run: `cargo test` (skip if no doc tooling)
Expected: PASS.

**Step 3: Commit**

```bash
git add README.md docs/getting-started.md
git commit -m "docs: add provisioning workflow"
```

---

## Execution Handoff

Plan complete and saved to `docs/plans/2026-02-06-provisioning-cli-impl.md`.

Two execution options:

1. **Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration
2. **Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

Which approach?
