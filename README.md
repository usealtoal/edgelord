# edgelord

> Finding edges like a true edgelord.

A Rust-based arbitrage detection and execution system for prediction markets.

## What This Does

Detects and exploits arbitrage opportunities on prediction markets:

1. **Single-condition** — When YES + NO < $1.00 (guaranteed profit)
2. **Market rebalancing** — When all outcome prices sum to less than $1.00

Based on research showing $40M in arbitrage profits extracted from Polymarket in one year.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    RUST CORE (tokio)                    │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────┐  │
│  │  WebSocket   │───▶│   Detector   │───▶│ Executor │  │
│  │   Handler    │    │   (domain)   │    │ (traits) │  │
│  └──────────────┘    └──────────────┘    └──────────┘  │
│         │                                      │        │
│         ▼                                      ▼        │
│  ┌──────────────┐                      ┌───────────┐   │
│  │  OrderBook   │                      │ Polymarket│   │
│  │    Cache     │                      │ Executor  │   │
│  └──────────────┘                      └───────────┘   │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

**Design principles:**
- **Domain-driven:** Exchange-agnostic core logic in `domain/`
- **Trait-based:** `ExchangeClient` and `OrderExecutor` traits enable multi-exchange support
- **Proper encapsulation:** Private fields with accessors, builder patterns
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
│   ├── ids.rs             # TokenId, MarketId (newtypes)
│   ├── money.rs           # Price, Volume (type aliases)
│   ├── market.rs          # MarketPair, MarketInfo
│   ├── orderbook.rs       # PriceLevel, OrderBook, OrderBookCache
│   ├── opportunity.rs     # Opportunity with builder pattern
│   ├── position.rs        # Position tracking
│   └── detector.rs        # Detection logic
│
├── exchange/              # Exchange abstraction layer
│   └── traits.rs          # ExchangeClient, OrderExecutor traits
│
└── polymarket/            # Polymarket implementation
    ├── client.rs          # REST API client
    ├── executor.rs        # Order execution (implements OrderExecutor)
    ├── websocket.rs       # WebSocket handler
    ├── messages.rs        # WS message types + domain conversion
    ├── registry.rs        # YES/NO market pair mapping
    └── types.rs           # API response types
```

## Tech Stack

- **Language:** Rust 2021 (maximum latency edge)
- **Async runtime:** tokio
- **CLOB client:** polymarket-client-sdk
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

**Phases 1-3 Complete** — Detection and execution working on testnet

- ✅ Phase 1: Foundation (WebSocket, market data)
- ✅ Phase 2: Detection (arbitrage scanner)
- ✅ Phase 3: Execution (order submission on Amoy testnet)
- 🔜 Phase 4: Risk management & Telegram alerts
- 🔜 Phase 5: Mainnet deployment

## References

- [Unravelling the Probabilistic Forest (arXiv:2508.03474)](https://arxiv.org/abs/2508.03474)
- [Arbitrage-Free Combinatorial Market Making (arXiv:1606.02825)](https://arxiv.org/abs/1606.02825)
- [Polymarket CLOB Docs](https://docs.polymarket.com/developers/CLOB/introduction)
- [rs-clob-client](https://github.com/Polymarket/rs-clob-client)

## Disclaimer

This is for educational purposes. Trading involves risk. Don't trade money you can't afford to lose. The authors of the referenced research extracted $40M; you probably won't.
