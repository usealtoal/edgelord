# Edgelord Architecture

> This document defines the structural principles for the codebase.
> All contributions must follow these rules.

## Naming Conventions

### Files & Modules

- **snake_case** for all files: `order_book.rs`, `market_score.rs`
- **Singular nouns** for modules: `domain/`, `exchange/`, `service/`
- **Underscores** for compound words: `order_book.rs` not `orderbook.rs`

### Exceptions

- Avoid generic plurals like `types.rs` or `traits.rs`. Use descriptive singular names instead (e.g., `exchange_config.rs`, `response.rs`, `stat.rs`).
- `statistics/` is allowed as a singular service name for aggregated stats.

### Types

| Suffix | Purpose | Example |
|--------|---------|---------|
| (none) | Traits | `Scorer`, `Filter`, `Solver` |
| `Config` | Configuration | `RiskConfig`, `PolymarketConfig` |
| `Manager` | Stateful coordinator | `SubscriptionManager` |
| `Builder` | Construction pattern | `AppBuilder` |
| `Strategy` | Detection algorithm | `SingleConditionStrategy` |

### Prefixes

- Exchange-specific types: `Polymarket*`, `Kalshi*`
- No prefix for exchange-agnostic types

## File Size Limits

- **Hard limit: 500 SLOC (source lines of code)**
- Tests (including `#[cfg(test)]` modules) **do not count** toward the limit
- Approaching limit? Split into submodule
- Prefer many small files over few large ones
- Exception: Generated code, test fixtures

## Module Depth

- **Maximum 3 levels**: `core/exchange/polymarket/`
- Never deeper - flatten or rethink abstraction
- If you need a 4th level, the abstraction is wrong

## Dependency Rules

```
cli → app → {exchange, strategy, service} → domain
strategy → solver
```

### Layer Responsibilities

| Layer | May Depend On | Responsibility |
|-------|---------------|----------------|
| `domain` | Nothing | Pure data types, no I/O |
| `exchange` | `domain` | Exchange abstraction & implementations |
| `strategy` | `domain`, `cache`, `solver` | Detection algorithms |
| `service` | `domain`, `cache`, `solver` | Runtime services (cluster detection, governor) |
| `solver` | `domain` | LP/ILP solver abstraction |
| `cache` | `domain` | Stateful caches (order books, clusters) |
| `llm` | `error` | LLM provider abstraction |
| `inference` | `domain`, `llm` | Relation inference |
| `store` | `domain`, `db` | Persistence abstraction |
| `db` | Nothing | Diesel ORM schema/models |
| `app` | All of `core` | Application orchestration, configuration |
| `cli` | `app` | User interface |

### Forbidden Dependencies

- `domain` cannot import from `exchange`, `strategy`, `service`, `app`, or `cli`
- `exchange`, `strategy`, `service` cannot import from each other
- `cli` cannot import directly from `core` (must go through `app`)

## Directory Structure

```
src/
├── main.rs                     # Entry point
├── lib.rs                      # Library root
├── error.rs                    # Error types
│
├── core/                       # Reusable library components
│   ├── mod.rs
│   │
│   ├── domain/                 # Pure business types
│   │   ├── mod.rs
│   │   ├── id.rs               # TokenId, MarketId, RelationId, ClusterId
│   │   ├── money.rs            # Price, Volume
│   │   ├── market.rs
│   │   ├── market_registry.rs
│   │   ├── order_book.rs
│   │   ├── opportunity.rs
│   │   ├── position.rs
│   │   ├── relation.rs         # Relation, RelationKind, Cluster
│   │   ├── resource.rs
│   │   ├── scaling.rs
│   │   └── score.rs
│   │
│   ├── exchange/               # Exchange abstraction
│   │   ├── mod.rs              # Core traits
│   │   ├── reconnecting.rs
│   │   └── <exchange>/         # Per-exchange implementation
│   │       ├── mod.rs
│   │       ├── client.rs
│   │       ├── websocket.rs
│   │       ├── executor.rs
│   │       ├── scorer.rs
│   │       ├── filter.rs
│   │       └── dedup.rs
│   │
│   ├── strategy/               # Detection algorithms
│   │   ├── mod.rs              # Strategy trait + registry
│   │   ├── context.rs
│   │   └── <category>/         # Per-category folder
│   │       ├── mod.rs
│   │       └── <variant>.rs
│   │
│   ├── service/                # Runtime services
│   │   ├── mod.rs
│   │   ├── cluster/            # Cluster detection service
│   │   │   ├── mod.rs          # ClusterDetectionService
│   │   │   └── detector.rs     # Detection logic
│   │   ├── subscription/
│   │   │   ├── mod.rs          # SubscriptionManager trait
│   │   │   └── priority.rs
│   │   ├── governor/
│   │   │   ├── mod.rs          # AdaptiveGovernor trait
│   │   │   └── latency.rs
│   │   ├── notification/
│   │   │   ├── mod.rs          # Notifier trait
│   │   │   └── telegram.rs
│   │   └── risk.rs
│   │
│   ├── solver/                 # LP/ILP abstraction
│   │   ├── mod.rs
│   │   └── highs.rs
│   │
│   ├── cache/                  # Stateful caches
│   │   ├── mod.rs
│   │   ├── order_book.rs
│   │   ├── cluster.rs          # ClusterCache for relations
│   │   └── position.rs
│   │
│   ├── llm/                    # LLM provider abstraction
│   │   ├── mod.rs              # Llm trait
│   │   ├── anthropic.rs
│   │   └── openai.rs
│   │
│   ├── inference/              # Relation inference
│   │   ├── mod.rs              # Inferrer trait
│   │   └── llm.rs              # LlmInferrer implementation
│   │
│   ├── store/                  # Persistence abstraction
│   │   ├── mod.rs              # Store traits
│   │   ├── sqlite.rs
│   │   └── memory.rs
│   │
│   └── db/                     # Database (Diesel ORM)
│       ├── mod.rs
│       ├── schema.rs
│       └── model.rs
│
├── app/                        # Application layer
│   ├── mod.rs
│   ├── orchestrator/           # Main event loop
│   │   ├── mod.rs
│   │   ├── handler.rs
│   │   └── execution.rs
│   ├── builder.rs              # AppBuilder
│   ├── state.rs
│   ├── status.rs
│   ├── statistics.rs
│   └── config/                 # Configuration
│       ├── mod.rs              # Main Config + load()
│       ├── profile.rs          # Profile, ResourceConfig
│       ├── strategy.rs         # Strategy configs
│       ├── service.rs          # Service configs
│       ├── logging.rs          # LoggingConfig
│       ├── llm.rs              # LlmConfig, provider settings
│       ├── inference.rs        # InferenceConfig
│       ├── cluster.rs          # ClusterDetectionConfig
│       └── <exchange>.rs       # Per-exchange config
│
└── cli/                        # Command-line interface
    ├── mod.rs
    ├── run.rs
    ├── status.rs
    ├── logs.rs
    ├── service.rs
    └── banner.rs
```

## Adding New Components

### New Exchange

1. Create `core/exchange/<name>/` with standard files:
   - `mod.rs` - Module exports
   - `client.rs` - REST API client
   - `websocket.rs` - WebSocket stream
   - `executor.rs` - Order execution
   - `scorer.rs` - MarketScorer implementation
   - `filter.rs` - MarketFilter implementation
   - `dedup.rs` - MessageDeduplicator implementation

2. Implement required traits:
   - `MarketDataStream`
   - `MarketFetcher`
   - `OrderExecutor`
   - `MarketScorer`
   - `MarketFilter`
   - `MessageDeduplicator`

3. Add configuration in `app/config/<name>.rs`

4. Register in `ExchangeFactory`

5. Add `Exchange::<Name>` variant to enum

### New Strategy

1. Determine category: `condition`, `rebalancing`, `combinatorial`, or create new

2. Create folder if new category: `core/strategy/<category>/`

3. Add variant file: `core/strategy/<category>/<variant>.rs`

4. Implement `Strategy` trait

5. Add config struct in `app/config/strategy.rs`

6. Register in `StrategyRegistry`

### New Service

1. Determine category: `subscription`, `governor`, `notification`, `risk`

2. Add to `core/service/<category>/`:
   - Trait in `mod.rs` (if new)
   - Implementation in `<name>.rs`

3. Add config in `app/config/service.rs`

4. Wire up in `app/builder.rs`

## Testing Patterns

### Unit Tests

- Colocated with implementation: `#[cfg(test)] mod tests { ... }`
- Test module at bottom of file
- Use `#[tokio::test]` for async tests

### Integration Tests

- Location: `tests/` directory
- One file per major feature area
- Naming: `<feature>_tests.rs`

### Test Utilities

- Shared fixtures in `tests/common/mod.rs`
- Mock implementations in `tests/mocks/`

## Configuration Patterns

### Layering

```
Profile (local/production)
    └── ResourceConfig (memory, threads)
        └── ServiceConfig (risk, governor, etc.)
            └── ExchangeConfig (per-exchange settings)
```

### Defaults

- All configs must implement `Default`
- Use `#[serde(default)]` for optional fields
- Document default values in doc comments

### Validation

- Validate in `Config::load()` after deserialization
- Return `ConfigError` for invalid values
- Validate relationships between fields

## Error Handling

- Use `thiserror` for error definitions
- Single `Error` enum in `src/error.rs`
- Categorize errors: `ConfigError`, `ExchangeError`, `StrategyError`
- Propagate with `?`, handle at boundaries

## Commit Messages

- Format: `<type>(<scope>): <description>`
- Types: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`
- Scopes: `domain`, `exchange`, `strategy`, `service`, `app`, `cli`, `config`
- One line, no co-authorship line
- Examples:
  - `feat(exchange): add Kalshi support`
  - `fix(strategy): correct spread calculation`
  - `refactor(config): split into modules`
