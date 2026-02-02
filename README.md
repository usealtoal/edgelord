# edgelord

> Finding edges like a true edgelord.

A Rust-based arbitrage detection and execution system for Polymarket prediction markets.

## What This Does

Detects and exploits three types of arbitrage on Polymarket:

1. **Single-condition** — When YES + NO ≠ $1.00
2. **Market rebalancing** — When outcome prices don't sum to $1.00
3. **Combinatorial** — When logical dependencies between markets create hidden profit

Based on research showing $40M in arbitrage profits extracted from Polymarket in one year.

## Architecture

```
WebSocket Feed → Detector → Executor
                    ↓
            Optimization Service (Gurobi)
```

See [doc/architecture/system-design.md](doc/architecture/system-design.md) for details.

## Tech Stack

- **Language:** Rust (maximum latency edge)
- **Async runtime:** tokio
- **CLOB client:** rs-clob-client (Polymarket official)
- **IP solver:** Gurobi (via grb crate)
- **Chain:** Polygon

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

🚧 **In Development**

## References

- [Unravelling the Probabilistic Forest (arXiv:2508.03474)](https://arxiv.org/abs/2508.03474)
- [Arbitrage-Free Combinatorial Market Making (arXiv:1606.02825)](https://arxiv.org/abs/1606.02825)
- [Polymarket CLOB Docs](https://docs.polymarket.com/developers/CLOB/introduction)
- [rs-clob-client](https://github.com/Polymarket/rs-clob-client)

## Disclaimer

This is for educational purposes. Trading involves risk. Don't trade money you can't afford to lose. The authors of the referenced research extracted $40M; you probably won't.
