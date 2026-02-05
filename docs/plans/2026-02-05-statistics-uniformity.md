# Statistics Uniformity Rename Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rename all `stats` app/CLI modules and commands to `statistics` for total uniformity.

**Architecture:** Keep core service as `core::service::statistics` and align the app façade and CLI command naming. This is a pure rename: no behavior changes, only module/file names and command identifiers.

**Tech Stack:** Rust, clap

---

### Task 1: Rename App Module to `statistics`

**Files:**
- Modify: `src/app/mod.rs`
- Rename: `src/app/stats.rs` → `src/app/statistics.rs`

**Step 1: Introduce compile failure by renaming the module declaration**

Edit `src/app/mod.rs`:
```rust
pub mod statistics;
```
Remove `pub mod stats;`.

**Step 2: Run a build to confirm failure**

Run:
```bash
cargo build
```
Expected: FAIL (module `statistics` not found).

**Step 3: Rename the app module file**

Run:
```bash
git mv src/app/stats.rs src/app/statistics.rs
```

**Step 4: Run a build to confirm it passes**

Run:
```bash
cargo build
```
Expected: PASS.

**Step 5: Commit**

```bash
git add src/app/mod.rs src/app/statistics.rs
git commit -m "refactor: rename app stats module to statistics"
```

---

### Task 2: Rename CLI Module + Types to `statistics`

**Files:**
- Modify: `src/cli/mod.rs`
- Modify: `src/cli/run.rs`
- Rename: `src/cli/stats.rs` → `src/cli/statistics.rs`

**Step 1: Introduce compile failure by renaming the module and command types**

In `src/cli/mod.rs`:
- Change `pub mod stats;` → `pub mod statistics;`
- Rename `StatsCommand` → `StatisticsCommand`
- Rename `StatsArgs` → `StatisticsArgs`
- Rename `StatsHistoryArgs` → `StatisticsHistoryArgs`
- Rename `StatsExportArgs` → `StatisticsExportArgs`
- Rename `StatsPruneArgs` → `StatisticsPruneArgs`
- Rename enum variant `Stats` → `Statistics`
- Update doc comments to reference `statistics`

**Step 2: Run a build to confirm failure**

Run:
```bash
cargo build
```
Expected: FAIL (missing module/file + type references).

**Step 3: Rename CLI file and fix imports**

Run:
```bash
git mv src/cli/stats.rs src/cli/statistics.rs
```

Update `src/cli/statistics.rs` to import `crate::app::statistics` instead of `crate::app::stats`, and update any references to `stats::` to `statistics::`.

**Step 4: Update CLI dispatch**

In `src/cli/run.rs`, update match arms and imports to use:
- `Commands::Statistics`
- `StatisticsCommand` and related argument types

**Step 5: Run a build to confirm it passes**

Run:
```bash
cargo build
```
Expected: PASS.

**Step 6: Commit**

```bash
git add src/cli/mod.rs src/cli/run.rs src/cli/statistics.rs
git commit -m "refactor: rename cli stats module to statistics"
```

---

### Task 3: Update Documentation for CLI Rename

**Files:**
- Modify: `ARCHITECTURE.md`
- Modify: any docs referencing `edgelord statistics`

**Step 1: Update architecture directory listing**

In `ARCHITECTURE.md`, replace:
```
├── stats.rs
```
with:
```
├── statistics.rs
```

**Step 2: Search for CLI command usage**

Run:
```bash
grep -R "edgelord statistics" -n README.md doc
```
Update matches to `edgelord statistics`.

**Step 3: Commit**

```bash
git add ARCHITECTURE.md README.md doc
git commit -m "docs: rename stats CLI references to statistics"
```

---

### Task 4: Final Verification

**Step 1: Run full build**

```bash
cargo build
```
Expected: PASS.

**Step 2: Commit (only if additional fixes were required)**

```bash
git add -A
git commit -m "chore: finalize statistics rename"
```

---

## Notes
- This change is intentionally breaking: the CLI command becomes `statistics` only.
- No alias or backward compatibility is added.
