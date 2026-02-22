# Edgelord Documentation

Edgelord is a Rust CLI for detecting and executing prediction-market arbitrage with configurable risk controls.

## Documentation Map

- [Getting Started](getting-started.md)
  - Install the binary, bootstrap config, provision a wallet, and run safely.
- [CLI Reference](cli-reference.md)
  - Command-by-command usage for `init`, `run`, `status`, `statistics`, `config`, `check`, `provision`, `wallet`, and `strategies`.
- [Configuration Reference](configuration.md)
  - Production-focused configuration guidance with practical defaults.
- [Testing Guide](testing.md)
  - Local test strategy and smoke-test guidance.
- [Architecture](architecture/overview.md)
  - High-level system design and runtime flow.
- [Strategy Guide](strategies/overview.md)
  - Detection strategies, constraints, and selection guidance.
- [Deployment](deployment/README.md)
  - VPS setup, wallet setup, Telegram integration, and operations.

## Intended Audience

- Operators deploying and running edgelord in a managed environment.
- Engineers extending strategy, exchange, or orchestration behavior.

## Conventions

- Paths are shown relative to repository root unless noted.
- Secrets are managed with [dugout](https://crates.io/crates/dugout) and injected at runtime.
- Environment variables are uppercase (for example, `WALLET_PRIVATE_KEY`).
- Commands assume a Unix-like shell.
