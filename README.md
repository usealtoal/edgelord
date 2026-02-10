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

Provision wallet (recommended):

```bash
export EDGELORD_KEYSTORE_PASSWORD="change-me"
./target/release/edgelord provision polymarket --config config.toml
```

Validate and run:

```bash
./target/release/edgelord check config --config config.toml
./target/release/edgelord check connection --config config.toml
./target/release/edgelord run --config config.toml
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
# Run
edgelord run --config config.toml

# Diagnostics
edgelord check live --config config.toml

# Wallet operations
edgelord wallet address --config config.toml
edgelord wallet approve --config config.toml --amount 1000 --yes

# Statistics
edgelord statistics today --db edgelord.db
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
