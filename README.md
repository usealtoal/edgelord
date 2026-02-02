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

```mermaid
flowchart TB
    subgraph core["RUST CORE (tokio)"]
        WS[WebSocket Handler] --> SR[Strategy Registry]
        SR --> RM[Risk Manager]
        WS --> OBC[OrderBook Cache]

        subgraph strategies[Strategies]
            SC[Single Condition]
            RB[Rebalancing]
            CB[Combinatorial]
        end

        SR --> strategies
        strategies --> HiGHS[HiGHS Solver<br/>LP/ILP]

        RM --> EX[Executor<br/>Polymarket]
        EX --> NR[Notifier Registry]

        subgraph notifiers[Notifiers]
            TG[Telegram]
            LOG[Log]
        end

        NR --> notifiers
    end
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
├── error.rs               # Structured error types
│
├── app/                   # Application layer
│   ├── config.rs          # Configuration loading
│   ├── orchestrator.rs    # Main application loop
│   └── state.rs           # Shared application state
│
├── domain/                # Exchange-agnostic core
│   ├── id.rs             # TokenId, MarketId (newtypes)
│   ├── money.rs           # Price, Volume (type aliases)
│   ├── market.rs          # MarketPair, MarketInfo
│   ├── orderbook.rs       # PriceLevel, OrderBook, OrderBookCache
│   ├── opportunity.rs     # Opportunity with builder pattern
│   ├── position.rs        # Position tracking
│   │
│   ├── strategy/          # Pluggable detection strategies
│   │   ├── single_condition.rs    # YES + NO < $1
│   │   ├── market_rebalancing.rs  # Sum of outcomes < $1
│   │   └── combinatorial/         # Frank-Wolfe + ILP
│   │
│   └── solver/            # LP/ILP solver abstraction
│       └── highs.rs       # HiGHS implementation
│
├── service/               # Cross-cutting services
│   ├── risk.rs            # RiskManager with limits & circuit breakers
│   ├── notifier.rs        # Notifier trait + registry
│   └── telegram.rs        # Telegram notifier (feature-gated)
│
├── exchange/              # Exchange abstraction layer
│   └── traits.rs          # ExchangeClient, OrderExecutor traits
│
└── adapter/               # Exchange implementations
    └── polymarket/        # Polymarket implementation
        ├── client.rs      # REST API client
        ├── executor.rs    # Order execution
        ├── websocket.rs   # WebSocket handler
        └── registry.rs    # YES/NO market pair mapping
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

[risk]
max_position_per_market = 1000   # $1000 max per market
max_total_exposure = 10000       # $10000 total portfolio limit
min_profit_threshold = 0.05      # $0.05 minimum profit
max_slippage = 0.02              # 2% maximum slippage

[telegram]
enabled = false                  # Set to true to enable
notify_opportunities = false     # Alert on new opportunities
notify_executions = true         # Alert on trade executions
notify_risk_rejections = true    # Alert when risk manager rejects
```

## Environment Variables

```bash
# Required for trading
export WALLET_PRIVATE_KEY="0x..."

# Required for Telegram notifications (when telegram feature enabled)
export TELEGRAM_BOT_TOKEN="your-bot-token"
export TELEGRAM_CHAT_ID="your-chat-id"
```

To enable Telegram notifications:
```bash
cargo build --features telegram
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

- [x] **Phase 1: Foundation** — WebSocket, market data, order book cache
- [x] **Phase 2: Detection** — Single-condition arbitrage scanner
- [x] **Phase 3: Execution** — Order submission on Amoy testnet
- [x] **Multi-Strategy** — Pluggable strategy system with Frank-Wolfe + ILP
- [x] **Phase 4: Risk & Alerts** — Risk manager, circuit breakers, Telegram notifications
- [ ] **Phase 5: Mainnet** — Production deployment with real funds

## References

- [Unravelling the Probabilistic Forest (arXiv:2508.03474)](https://arxiv.org/abs/2508.03474)
- [Arbitrage-Free Combinatorial Market Making (arXiv:1606.02825)](https://arxiv.org/abs/1606.02825)
- [Polymarket CLOB Docs](https://docs.polymarket.com/developers/CLOB/introduction)
- [HiGHS LP Solver](https://highs.dev/)

## Disclaimer

This is for educational purposes. Trading involves risk. Don't trade money you can't afford to lose. The authors of the referenced research extracted $40M; you probably won't.
