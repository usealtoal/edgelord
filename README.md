# edgelord

Arbitrage detection for prediction markets.

## Install

    cargo install edgelord

## Quick start

    edgelord init
    edgelord check live
    edgelord run

## What it does

Detects and executes arbitrage opportunities on Polymarket.
Three strategies ship by default:

| Strategy | Signal | Typical edge |
|----------|--------|--------------|
| single-condition | YES + NO < $1 | 2-5% |
| market-rebalancing | sum(outcomes) < $1 | 1-3% |
| combinatorial | cross-market constraints | <1% |

## Commands

    edgelord init              Setup wizard
    edgelord run               Start trading
    edgelord status            Show current state
    edgelord strategies list   Available strategies
    edgelord check live        Validate config
    edgelord wallet status     Show approvals

Run `edgelord --help` for all commands.

## Configuration

See [docs/configuration.md](docs/configuration.md).

## Extending

Fork this repo. Implement `ports::Strategy`. See
[docs/strategies/overview.md](docs/strategies/overview.md).

## Architecture

```
domain/     Pure types, no I/O
ports/      Trait definitions (extension points)
adapters/   Implementations (Polymarket, strategies, etc.)
runtime/    Orchestration and wiring
cli/        Command-line interface
```

## License

MIT OR Apache-2.0
