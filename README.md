# edgelord

> Finding edges like a true edgelord.

A Rust-based multi-strategy arbitrage detection and execution system for prediction markets.

## What This Does

Detects and exploits arbitrage opportunities on prediction markets using pluggable detection strategies:

| Strategy | Description | Historical Profit Share |
|----------|-------------|------------------------|
| **Single-condition** | YES + NO < $1.00 | 26.7% ($10.5M) |
| **Market rebalancing** | Sum of all outcomes < $1.00 | 73.1% ($29M) |
| **Combinatorial** | Frank-Wolfe + ILP for correlated markets | 0.24% ($95K) |

Based on research showing $40M in arbitrage profits extracted from Polymarket in one year.

## Architecture

```
┌───────────────────────────────────────────────────────────────────┐
│                      RUST CORE (tokio)                            │
├───────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐        │
│  │  WebSocket   │───▶│   Strategy   │───▶│   Executor   │        │
│  │   Handler    │    │   Registry   │    │   (traits)   │        │
│  └──────────────┘    └──────────────┘    └──────────────┘        │
│         │                   │                    │                │
│         ▼                   ▼                    ▼                │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐        │
│  │  OrderBook   │    │  Strategies  │    │  Polymarket  │        │
│  │    Cache     │    │  ┌─────────┐ │    │   Executor   │        │
│  └──────────────┘    │  │ Single  │ │    └──────────────┘        │
│                      │  │Condition│ │                             │
│                      │  ├─────────┤ │                             │
│                      │  │Rebalanc.│ │                             │
│                      │  ├─────────┤ │                             │
│                      │  │Combinat.│ │                             │
│                      │  └─────────┘ │                             │
│                      └──────────────┘                             │
│                             │                                     │
│                             ▼                                     │
│                      ┌──────────────┐                             │
│                      │ HiGHS Solver │                             │
│                      │  (LP/ILP)    │                             │
│                      └──────────────┘                             │
│                                                                   │
└───────────────────────────────────────────────────────────────────┘
```

**Design principles:**
- **Strategy pattern:** Pluggable detection algorithms via `Strategy` trait
- **Domain-driven:** Exchange-agnostic core logic in `domain/`
- **Solver abstraction:** Swappable LP/ILP backends (HiGHS by default)
- **Type safety:** Newtypes for identifiers, Decimal for money (never floats)

See [doc/architecture/system-design.md](doc/architecture/system-design.md) for details.

## Project Structure

```
src/
├── lib.rs                 # Library root with public API
├── main.rs                # Thin binary entry point
├── app.rs                 # Application orchestration
├── config.rs              # Configuration loading
├── error.rs               # Structured error types
│
├── domain/                # Exchange-agnostic core
│   ├── id.rs             # TokenId, MarketId (newtypes)
│   ├── money.rs           # Price, Volume (type aliases)
│   ├── market.rs          # MarketPair, MarketInfo
│   ├── orderbook.rs       # PriceLevel, OrderBook, OrderBookCache
│   ├── opportunity.rs     # Opportunity with builder pattern
│   ├── position.rs        # Position tracking
│   ├── detector.rs        # Legacy re-export (use strategy/)
│   │
│   ├── strategy/          # Pluggable detection strategies
│   │   ├── mod.rs         # Strategy trait + StrategyRegistry
│   │   ├── context.rs     # DetectionContext, MarketContext
│   │   ├── single_condition.rs    # YES + NO < $1
│   │   ├── market_rebalancing.rs  # Sum of outcomes < $1
│   │   └── combinatorial/         # Frank-Wolfe + ILP
│   │       ├── mod.rs             # CombinatorialStrategy
│   │       ├── bregman.rs         # Bregman divergence (KL)
│   │       └── frank_wolfe.rs     # Frank-Wolfe algorithm
│   │
│   └── solver/            # LP/ILP solver abstraction
│       ├── mod.rs         # Solver trait + types
│       └── highs.rs       # HiGHS implementation
│
├── exchange/              # Exchange abstraction layer
│   └── traits.rs          # ExchangeClient, OrderExecutor traits
│
└── polymarket/            # Polymarket implementation
    ├── client.rs          # REST API client
    ├── executor.rs        # Order execution
    ├── websocket.rs       # WebSocket handler
    ├── messages.rs        # WS message types
    ├── registry.rs        # YES/NO market pair mapping
    └── types.rs           # API response types
```

## Configuration

```toml
[strategies]
enabled = ["single_condition", "market_rebalancing"]

[strategies.single_condition]
min_edge = 0.05      # 5% minimum edge
min_profit = 0.50    # $0.50 minimum profit

[strategies.market_rebalancing]
min_edge = 0.03      # 3% minimum edge
min_profit = 1.00    # $1.00 minimum profit
max_outcomes = 10    # Skip markets with >10 outcomes

[strategies.combinatorial]
enabled = false      # Requires dependency configuration
max_iterations = 20
tolerance = 0.0001
gap_threshold = 0.02
```

## Tech Stack

- **Language:** Rust 2021 (maximum latency edge)
- **Async runtime:** tokio
- **LP/ILP Solver:** HiGHS via good_lp
- **Decimals:** rust_decimal (never floats for money)
- **Chain:** Polygon (mainnet) / Amoy (testnet)

## Documentation

```
doc/
├── research/
│   ├── polymarket-arbitrage.md   # The math and strategy
│   └── polymarket-technical.md   # API and infrastructure
├── architecture/
│   └── system-design.md          # System architecture
└── plans/
    └── (implementation plans)
```

## Status

**Multi-Strategy Architecture Complete**

- ✅ Phase 1: Foundation (WebSocket, market data)
- ✅ Phase 2: Detection (single-condition arbitrage scanner)
- ✅ Phase 3: Execution (order submission on Amoy testnet)
- ✅ Multi-Strategy: Pluggable strategy system with Frank-Wolfe + ILP
- 🔜 Phase 4: Risk management & Telegram alerts
- 🔜 Phase 5: Mainnet deployment

## References

- [Unravelling the Probabilistic Forest (arXiv:2508.03474)](https://arxiv.org/abs/2508.03474)
- [Arbitrage-Free Combinatorial Market Making (arXiv:1606.02825)](https://arxiv.org/abs/1606.02825)
- [Polymarket CLOB Docs](https://docs.polymarket.com/developers/CLOB/introduction)
- [HiGHS LP Solver](https://highs.dev/)

## Disclaimer

This is for educational purposes. Trading involves risk. Don't trade money you can't afford to lose. The authors of the referenced research extracted $40M; you probably won't.
