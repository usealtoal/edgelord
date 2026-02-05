# Edgelord Consistency + Test Cleanup Design

**Date:** 2026-02-05

## Goals
- Bring code and documentation back into alignment with the architecture rules.
- Enforce naming and module structure consistency without unnecessary churn.
- Establish a clean, reusable test helper layout and consistent test naming.
- Reduce duplication in tests while keeping behavior coverage intact.

## Non-Goals
- No behavior changes to trading logic, strategy math, or execution semantics.
- No re-architecture of services or strategy algorithms beyond dependency cleanup.
- No large-scale API surface changes outside required refactors.

## Decisions
### Architecture + Dependency Rules
- Update `ARCHITECTURE.md` to reflect that `app/orchestrator/` is the correct structure.
- Clarify the file-size rule is **SLOC** (tests excluded).
- Introduce a narrow exception for `types.rs` (Rust keyword conflict).
- Move Frank-Wolfe implementation into `core/solver` so both `service` and `strategy` depend on `solver` without crossing layers.
- Update the dependency rules to allow `strategy -> solver` explicitly.

### Naming + Module Layout
- Rename plural modules where reasonable to match singular-noun rule:
  - `core/service/stats/` -> `core/service/statistics/`
  - `core/exchange/polymarket/messages.rs` -> `message.rs`
- Keep stable exports via `pub use` to reduce downstream disruption.

### CLI Layering
- Introduce `app` façade modules for stats/status so CLI uses `app` instead of `core::db` or `core::service` directly.

### Tests + Helpers
- Add `tests/support/` with a focused helper layout:
  - `tests/support/mod.rs` (re-exports)
  - `tests/support/market.rs`
  - `tests/support/order_book.rs`
  - `tests/support/registry.rs`
  - `tests/support/relation.rs`
  - `tests/support/config.rs`
  - `tests/support/assertions.rs`
- Normalize integration test names to `*_tests.rs`.
- Move pure logic tests into unit tests under the relevant modules where possible.
- Replace time-based sleeps with deterministic coordination where feasible.

## Implementation Outline
1. Update `ARCHITECTURE.md` (orchestrator folder, SLOC rule, solver dependency, types.rs exception).
2. Move Frank-Wolfe to `core/solver/` and adjust imports in `strategy` and `service`.
3. Rename modules to singular where applicable and update re-exports.
4. Add `app::status` and `app::stats` façade modules; update CLI imports to go through `app`.
5. Create `tests/support/` helpers and refactor integration tests to use them.
6. Normalize test file names to `*_tests.rs` and relocate pure unit tests.
7. Run `cargo test` after refactors and verify no behavior regressions.

## Risks + Mitigations
- **Path churn from renames:** use `pub use` re-exports to preserve external paths.
- **Layering rule conflicts:** update doc and code together to keep single source of truth.
- **Test flakiness during refactor:** convert sleeps to deterministic coordination as part of test cleanup.

## Verification
- `cargo test` (full suite)
- Spot-check key integration tests: strategy, cluster detection, exchange, CLI
