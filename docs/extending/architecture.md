# Architecture

Edgelord uses hexagonal architecture with clear separation between
domain logic, ports (interfaces), and adapters (implementations).

## Module Structure

```
src/
├── domain/      Pure types. No I/O, no dependencies.
├── ports/       Trait definitions. Extension points.
├── adapters/    Implementations. Polymarket, strategies, etc.
├── runtime/     Orchestration. Wires components together.
└── cli/         Command-line interface.
```

## Dependency Rules

- `domain/` imports nothing
- `ports/` imports only `domain/`
- `adapters/` imports `domain/` and `ports/`
- `runtime/` imports all above
- `cli/` imports `runtime/`

## Key Extension Points

| Port | Purpose |
|------|---------|
| `Strategy` | Detection algorithms |
| `MarketDataStream` | Real-time data feeds |
| `ArbitrageExecutor` | Order execution |
| `Notifier` | Event notifications |
| `Store` | Persistence |
| `Solver` | LP/ILP optimization |

## Adding Features

1. Define trait in `ports/`
2. Implement in `adapters/`
3. Wire in `runtime/`
