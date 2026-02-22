# edgelord

Arbitrage detection for prediction markets.

## Install

    cargo install edgelord

## Quick start

    edgelord init
    edgelord check config
    edgelord check health
    edgelord check live
    edgelord run

## What it does

Detects and executes arbitrage opportunities on Polymarket.
Three strategies ship by default:

| Strategy | Signal | Typical edge |
|----------|--------|--------------|
| single_condition | YES + NO < $1 | 2-5% |
| market_rebalancing | sum(outcomes) < $1 | 1-3% |
| combinatorial | cross-market constraints | <1% |

## Commands

    edgelord init              Setup wizard
    edgelord run               Start trading
    edgelord status            Show current state
    edgelord strategies list   Available strategies
    edgelord check config      Validate config
    edgelord check health      Local health checks
    edgelord check live        Live readiness checks
    edgelord wallet status     Show approvals

Run `edgelord --help` for all commands.

## Configuration

See [docs/configuration.md](docs/configuration.md).

## Extending

Fork this repo. Implement `port::inbound::strategy::Strategy`. See
[docs/strategies/overview.md](docs/strategies/overview.md).

## Architecture

```
domain/         Pure types and runtime state, no external integrations
port/           Contracts split by direction (inbound/, outbound/)
adapter/        Adapters split by direction (inbound/cli, outbound/integrations)
application/    Use-case orchestration and business flow
infrastructure/ Wiring, config, bootstrap, and runtime facades
```

CLI commands call focused `port/inbound/operator/*` capabilities (configuration, diagnostics, runtime, status, statistics, wallet) through an injected operator capability surface.

## License

MIT OR Apache-2.0
