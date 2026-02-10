<div align="center">
  <img src="asset/banner.png" alt="edgelord" width="100%">

  <p><strong>Multi-strategy arbitrage detection and execution for prediction markets, written in Rust</strong></p>

  <p>
    <a href="https://github.com/usealtoal/edgelord/actions/workflows/ci.yml"><img src="https://github.com/usealtoal/edgelord/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
    <img src="https://img.shields.io/badge/license-proprietary-lightgrey.svg" alt="License">
    <img src="https://img.shields.io/badge/rust-1.75%2B-orange.svg" alt="Rust">
  </p>

</div>

## Overview

edgelord is a Rust CLI for running arbitrage detection and execution workflows against prediction-market exchanges.

Current implementation focus:

- Exchange: Polymarket
- Detection model: multi-strategy (single-condition, market-rebalancing, combinatorial)
- Runtime model: event-driven with risk-gated execution

## Strategy Coverage

| Strategy | Market Scope | Core Signal |
|---|---|---|
| Market Rebalancing | Multi-outcome markets | `sum(outcomes) < payout` |
| Single-Condition | Binary markets | `YES + NO < payout` |
| Combinatorial | Related market clusters | Cross-market constraint violations |

## Quick Start

```bash
git clone https://github.com/usealtoal/edgelord.git
cd edgelord
cargo build --release
cp config.toml.example config.toml
```

Set up secrets with [dugout](https://crates.io/crates/dugout):

```bash
cargo install dugout
dugout init
dugout set WALLET_PRIVATE_KEY
```

Validate and run:

```bash
dugout run -- ./target/release/edgelord check config --config config.toml
dugout run -- ./target/release/edgelord check connection --config config.toml
dugout run -- ./target/release/edgelord run --config config.toml
```

## Production Readiness Flow

1. Run in `dry_run = true` first.
2. Validate with `check live` before any live deployment.
3. Start with conservative risk limits.
4. Promote to mainnet only after stable observation windows.

## Documentation

- [Documentation Home](docs/README.md)
- [Getting Started](docs/getting-started.md)
- [CLI Reference](docs/cli-reference.md)
- [Configuration Reference](docs/configuration.md)
- [Strategy Guide](docs/strategies/overview.md)
- [Architecture](docs/architecture/overview.md)
- [Deployment Guide](docs/deployment/README.md)
- [Testing Guide](docs/testing.md)

## Example Commands

```bash
# Run (with secrets via dugout)
dugout run -- edgelord run --config config.toml

# Or spawn a shell with secrets loaded
dugout env
edgelord run --config config.toml

# Diagnostics
dugout run -- edgelord check live --config config.toml

# Wallet operations (need secrets)
dugout run -- edgelord wallet address --config config.toml
dugout run -- edgelord wallet approve --config config.toml --amount 1000 --yes

# Statistics (no secrets needed)
edgelord statistics today --db edgelord.db
edgelord logs -f
```

## Project Structure

```text
src/
├── app/      # Orchestration and config loading
├── cli/      # Command handlers and CLI surface
└── core/     # Domain, exchange adapters, strategies, services, solvers
```

## License

Proprietary. All rights reserved.
