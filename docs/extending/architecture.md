# Architecture

Edgelord uses hexagonal architecture with directional ports and adapters.

## Module Structure

```
src/
├── domain/                 Pure types and invariants (no I/O)
├── port/
│   ├── inbound/            Use-case capability contracts
│   └── outbound/           Infrastructure dependency contracts
├── application/            Use-case orchestration
├── adapter/
│   ├── inbound/cli/        CLI entrypoints and command handlers
│   └── outbound/           Exchange/storage/notifier/solver/llm implementations
└── infrastructure/         Wiring, runtime orchestration, bootstrap
```

## Dependency Rules

- `domain/` imports nothing from crate layers
- `port/` imports only `domain/`
- `application/` imports only `domain/` and `port/`
- `adapter/outbound/` imports only `domain/` and `port/`
- `adapter/inbound/` never imports `adapter/outbound/` directly
- `adapter/inbound/` does not import `infrastructure/` directly
- `infrastructure/` owns composition/wiring across layers

## Key Extension Points

| Port | Purpose |
|------|---------|
| `port/inbound/strategy` | Strategy capability surface |
| `port/inbound/operator/*` | CLI/operator capability surfaces (configuration, diagnostics, runtime, status, statistics, wallet) |
| `port/outbound/exchange` | Market data + execution integration |
| `port/outbound/notifier` | Event notifications |
| `port/outbound/store` | Persistence |
| `port/outbound/solver` | Optimization backend |

## Adding Features

1. Define or extend a contract in `port/` when needed.
2. Implement business flow in `application/`.
3. Implement driven integrations in `adapter/outbound/`.
4. Add/extend command handling in `adapter/inbound/cli/`.
5. Wire concrete dependencies in `infrastructure/`.
