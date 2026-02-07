# Testing Guide

Edgelord uses layered testing:

- Unit tests for domain/service correctness
- Integration tests for orchestration and CLI behavior
- Optional live smoke checks for external dependencies

## Standard Test Workflow

```bash
cargo fmt --all -- --check
cargo test
```

## Targeted Suites

Examples:

```bash
cargo test --test e2e_flow_tests
cargo test --test cli_provision_tests
cargo test --test cli_wallet_tests
cargo test --test exchange_tests
```

## Live Smoke Tests

Live smoke checks are ignored by default and should be run intentionally.

```bash
cargo test -- --ignored --nocapture
EDGELORD_SMOKE=1 cargo test -- --ignored --nocapture
```

Guidance:

- Keep smoke tests read-only unless explicitly designed for controlled execution.
- Use dedicated credentials/wallets for any non-read smoke coverage.
- Treat smoke tests as environment validation, not a substitute for deterministic tests.

## CI Expectations

Before opening a PR, ensure:

1. Formatting is clean (`cargo fmt --check`)
2. Full suite passes (`cargo test`)
3. New behavior has deterministic tests where practical
