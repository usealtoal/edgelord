# Repository Restructure Design

> Status: Historical (superseded)
> Superseded by: 2026-02-03-repository-restructure-impl.md
> Summary:
> - Scope: Megalithic Files
> Planned Outcomes:
> - Megalithic Files
> - Structural Issues


## Goal

Establish a durable, well-organized folder/file structure that:
- Enforces consistent naming patterns
- Prevents megalithic files
- Scales with new exchanges and strategies
- Requires minimal future reorganization

## Current Issues

### Megalithic Files

| File | Lines | Problem |
|------|-------|---------|
| `app/config.rs` | 983 | 31+ config structs in one file |
| `service/priority_subscription.rs` | 753 | Multiple responsibilities |
| `service/latency_governor.rs` | 628 | Could extract utilities |
| `app/orchestrator.rs` | 592 | Initialization + event loop |
| `strategy/market_rebalancing.rs` | 556 | LP construction embedded |
| `strategy/single_condition.rs` | 423 | Approaching limit |

### Structural Issues

1. **Flat service module** - 7 files at same level with different concerns
2. **Missing config organization** - All configs in single file
3. **Inconsistent naming** - `orderbook.rs` vs `order_book.rs` pattern
4. **No strategy folders** - Strategies as flat files, not categorized

## Design Decisions

### 1. Naming Conventions

- **Singular modules**: `domain/`, `exchange/`, `service/`
- **snake_case files**: `order_book.rs`, `market_score.rs`
- **Type suffixes**: `*Config`, `*Manager`, `*Builder`, `*Strategy`
- **Exchange prefixes**: `Polymarket*`, `Kalshi*`

### 2. File Size Limit

- **Hard limit: 400 lines**
- Split into submodule when approaching
- Prefer many small files

### 3. Module Depth

- **Maximum 3 levels**: `core/exchange/polymarket/`
- Never deeper

### 4. Dependency Flow

```
cli → app → {exchange, strategy, service} → domain
```

No horizontal dependencies between exchange/strategy/service.

## Target Structure

```
src/
├── main.rs
├── lib.rs
├── error.rs
│
├── core/
│   ├── domain/                 # (unchanged)
│   │   └── order_book.rs       # RENAME from orderbook.rs
│   │
│   ├── exchange/
│   │   ├── mod.rs              # Traits only
│   │   ├── reconnecting.rs
│   │   └── polymarket/         # (unchanged)
│   │
│   ├── strategy/
│   │   ├── mod.rs
│   │   ├── context.rs
│   │   ├── condition/          # NEW folder
│   │   │   ├── mod.rs
│   │   │   └── single.rs       # FROM single_condition.rs
│   │   ├── rebalancing/        # NEW folder
│   │   │   ├── mod.rs
│   │   │   └── problem.rs      # LP construction extracted
│   │   └── combinatorial/      # (unchanged)
│   │
│   ├── service/
│   │   ├── mod.rs
│   │   ├── subscription/       # NEW folder
│   │   │   ├── mod.rs
│   │   │   └── priority.rs
│   │   ├── governor/           # NEW folder
│   │   │   ├── mod.rs
│   │   │   └── latency.rs
│   │   ├── notification/       # NEW folder
│   │   │   ├── mod.rs
│   │   │   └── telegram.rs
│   │   └── risk.rs
│   │
│   ├── solver/                 # (unchanged)
│   └── cache/
│       └── order_book.rs       # RENAME from orderbook.rs
│
├── app/
│   ├── mod.rs
│   ├── orchestrator.rs         # Slimmed down
│   ├── builder.rs              # NEW: extracted initialization
│   ├── state.rs
│   ├── status.rs               # RENAME from status_file.rs
│   └── config/                 # NEW folder
│       ├── mod.rs
│       ├── profile.rs
│       ├── strategy.rs
│       ├── service.rs
│       ├── logging.rs
│       └── polymarket.rs
│
└── cli/                        # (unchanged)
```

## Changes Summary

### Renames (4)

| From | To |
|------|-----|
| `domain/orderbook.rs` | `domain/order_book.rs` |
| `cache/orderbook.rs` | `cache/order_book.rs` |
| `app/status_file.rs` | `app/status.rs` |
| `strategy/single_condition.rs` | `strategy/condition/single.rs` |

### Splits (3)

| From | To |
|------|-----|
| `app/config.rs` (983 lines) | `app/config/` (6 files) |
| `strategy/market_rebalancing.rs` | `strategy/rebalancing/` (2 files) |
| `app/orchestrator.rs` | `orchestrator.rs` + `builder.rs` |

### Reorganizations (3)

| From | To |
|------|-----|
| `service/subscription.rs` | `service/subscription/mod.rs` |
| `service/priority_subscription.rs` | `service/subscription/priority.rs` |
| `service/governor.rs` | `service/governor/mod.rs` |
| `service/latency_governor.rs` | `service/governor/latency.rs` |
| `service/notifier.rs` | `service/notification/mod.rs` |
| `service/telegram.rs` | `service/notification/telegram.rs` |

### New Files (2)

| File | Purpose |
|------|---------|
| `ARCHITECTURE.md` | Codified principles |
| `app/builder.rs` | AppBuilder for initialization |

## Implementation Order

1. Create `ARCHITECTURE.md` (principles document)
2. Rename files (simple, low risk)
3. Split `app/config.rs` into module (biggest change)
4. Reorganize `service/` into submodules
5. Split `strategy/` files into folders
6. Extract `app/builder.rs` from orchestrator
7. Update all imports and re-exports
8. Run tests, fix any issues

## Success Criteria

- All files under 400 lines
- Maximum 3 levels of nesting
- Consistent naming throughout
- All tests pass
- Clean `cargo clippy`
