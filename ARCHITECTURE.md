# Edgelord Architecture

This worktree follows strict hexagonal architecture with explicit directional boundaries.

## Layer Model

```text
adapter/inbound (CLI)
  -> port/inbound (capability contracts)
    -> application (use-case orchestration)
      -> port/outbound (driven contracts)
        -> adapter/outbound (driven implementations)
infrastructure owns composition + runtime entrypoints.

application also defines capabilities behind port/inbound contracts.
domain remains pure and dependency-free.
```

## Dependency Rules

### Allowed

- `domain`: no dependencies on other crate layers.
- `port`: depends on `domain` only.
- `application`: depends on `domain` + `port`.
- `adapter/outbound`: depends on `domain` + `port`.
- `adapter/inbound`: depends on `port/inbound` contracts and request/response DTOs.
- `infrastructure`: owns wiring and runtime composition.

### Forbidden

- `domain -> port|application|adapter|infrastructure`
- `port -> application|adapter|infrastructure`
- `application -> adapter|infrastructure` (except test-only seams)
- `adapter/outbound -> application|infrastructure`
- `adapter/inbound -> adapter/outbound`
- `adapter/outbound -> adapter/inbound`

## Source Layout

```text
src/
├── adapter/
│   ├── inbound/
│   │   └── cli/
│   │       ├── check/
│   │       ├── provision/
│   │       ├── wallet/
│   │       └── *.rs command handlers
│   └── outbound/
│       ├── inference/
│       ├── llm/
│       ├── memory.rs
│       ├── notifier/
│       ├── polymarket/
│       ├── solver/
│       └── sqlite/
│           └── database/
│
├── application/
│   ├── cache/
│   ├── cluster/
│   ├── inference/
│   ├── orchestration/
│   ├── position/
│   ├── risk/
│   ├── solver/
│   ├── state.rs
│   └── strategy/
│
├── domain/
│   └── *.rs domain model modules
│
├── infrastructure/
│   ├── bootstrap.rs
│   ├── config/
│   ├── exchange/
│   ├── governor/
│   ├── orchestration/
│   ├── subscription/
│   └── wallet.rs
│
└── port/
    ├── inbound/
    │   ├── operator/
    │   │   ├── configuration.rs
    │   │   ├── diagnostic.rs
    │   │   ├── port.rs
    │   │   ├── runtime.rs
    │   │   ├── statistics.rs
    │   │   ├── status.rs
    │   │   └── wallet.rs
    │   ├── risk.rs
    │   ├── runtime.rs
    │   └── strategy.rs
    └── outbound/
        ├── approval.rs
        ├── dedup.rs
        ├── exchange.rs
        ├── filter.rs
        ├── inference.rs
        ├── llm.rs
        ├── notifier.rs
        ├── report.rs
        ├── solver.rs
        ├── stats.rs
        └── store.rs
```

## Naming Rules

- Folders/modules are singular where role-based singular names are clearer.
- File names are `snake_case` and role-specific.
- `mod.rs` files only declare/compose that directory level.
- Keep SRP per module: one reason to change per file/module.

## Extension Rules

1. New exchange: add under `adapter/outbound/<exchange>/`, implement `port/outbound/exchange` traits including `MarketParser`, wire in `infrastructure/exchange/factory.rs`.
2. New strategy: add under `application/strategy/`, implement `port/inbound/strategy` contract, wire in `application/strategy/registry.rs`.
3. New persistence integration: implement `port/outbound/store` and `port/outbound/stats` in `adapter/outbound`, wire in `infrastructure/bootstrap.rs`.
4. New CLI command: add under `adapter/inbound/cli/` and route from `src/main.rs`.
