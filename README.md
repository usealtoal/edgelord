<p align="center">
  <img src="assets/banner.png" alt="edgelord" width="1000">
</p>

<p align="center">
  <em>A prediction market arbitrage detection and execution CLI, written in Rust.</em>
</p>

<p align="center">
  <a href="https://github.com/usealtoal/edgelord/actions"><img src="https://github.com/usealtoal/edgelord/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://crates.io/crates/edgelord"><img src="https://img.shields.io/crates/v/edgelord.svg" alt="Crates.io"></a>
  <a href="https://github.com/usealtoal/edgelord/blob/main/LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue" alt="License"></a>
</p>

---

> I built this to test arbitrage conditions on Polymarket, but it doubled as an experiment in pure hexagonal architecture. The three strategies *do* work—arbitrage is hard (latency, liquidity, market selection all matter), but the math is sound. Run this against 10,000+ markets concurrently and you'll find opportunities. If you're curious about the mechanics, I wrote up [how each strategy works](docs/strategies/overview.md).

---

## Highlights

- **Real-time arbitrage detection** across binary and multi-outcome markets
- **Three strategies** out of the box: single-condition, market rebalancing, combinatorial
- **LLM-powered inference** for cross-market constraint discovery
- **Risk management** with position limits, exposure caps, and circuit breakers
- **Telegram notifications** for trades, opportunities, and alerts
- **SQLite persistence** for statistics, trades, and historical analysis

## Installation

With the standalone installer:

```bash
curl -LsSf https://raw.githubusercontent.com/usealtoal/edgelord/main/scripts/install.sh | sh
```

With [Homebrew](https://brew.sh):

```bash
brew install usealtoal/tap/edgelord
```

With [cargo](https://doc.rust-lang.org/cargo/):

```bash
cargo install edgelord
```

## Quick Start

```console
$ edgelord init
$ edgelord check config
$ edgelord check live
$ edgelord run
```

## Strategies

| Strategy | Signal | Typical Edge |
|----------|--------|--------------|
| `single_condition` | YES + NO < $1 | 2–5% |
| `market_rebalancing` | sum(outcomes) < $1 | 1–3% |
| `combinatorial` | cross-market constraints | <1% |

```console
$ edgelord strategies list
$ edgelord strategies explain single_condition
```

## Commands

```console
$ edgelord init                  # Setup wizard
$ edgelord run                   # Start trading
$ edgelord status                # Current state and today's P&L
$ edgelord statistics today      # Today's statistics
$ edgelord statistics week       # 7-day summary
$ edgelord check config          # Validate configuration
$ edgelord check live            # Live readiness checks
$ edgelord wallet status         # Token approvals
$ edgelord wallet approve 1000   # Approve $1000 for trading
```

See `edgelord --help` for all commands.

## Configuration

Create a config file with `edgelord init` or manually:

```toml
[exchange]
provider = "polymarket"
network = "polygon"

[wallet]
private_key_env = "PRIVATE_KEY"

[risk]
max_position_per_market = 100
max_total_exposure = 1000
min_profit_threshold = 0.50

[strategies.single_condition]
enabled = true
min_edge = 0.05

[telegram]
enabled = true
bot_token_env = "TELEGRAM_BOT_TOKEN"
chat_id_env = "TELEGRAM_CHAT_ID"
```

See [docs/configuration.md](docs/configuration.md) for full reference.

## Architecture

```
domain/         Pure types, no external dependencies
port/           Inbound and outbound contracts
adapter/        CLI, exchange integrations, notifications
application/    Use-case orchestration
infrastructure/ Config, bootstrap, runtime wiring
```

Hexagonal architecture with clean separation between domain logic and external integrations.

## Extending

Implement `port::inbound::strategy::Strategy` to add custom strategies:

```rust
impl Strategy for MyStrategy {
    fn detect(&self, ctx: &StrategyContext) -> Vec<Opportunity> {
        // Your detection logic
    }
}
```

See [docs/strategies/overview.md](docs/strategies/overview.md).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
