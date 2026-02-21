# Edgelord Open-Source Framework Design

## Vision

A beautifully crafted Rust CLI and framework for building prediction market trading bots. Ships with Polymarket support and three arbitrage strategies out of the box.

**Target audiences:**
- Traders: `cargo install edgelord`, configure, run. No Rust knowledge needed.
- Developers: Fork, extend strategies, add exchanges. Clean architecture to learn from.

**Core principles:**
- CLI-first. The binary is the product. Config files and commands, not library imports.
- Fork-friendly. Clean module boundaries, public traits, documented extension points.
- Astral-grade UX. Fast startup, beautiful output, excellent errors, self-documenting.
- Production-ready. Risk controls, observability, systemd support from day one.

## What Ships in v1

- Single `edgelord` crate published to crates.io
- All three strategies (single-condition, rebalancing, combinatorial)
- Polymarket exchange support (mainnet and testnet)
- LLM-based cluster inference (Anthropic, OpenAI)
- Telegram notifications
- SQLite statistics and position tracking
- Risk management (exposure limits, circuit breakers)
- Adaptive scaling/governor
- Full CLI with guided onboarding

**Non-goals for v1:**
- Multi-exchange support (architecture supports it, but only Polymarket ships)
- Plugin system or dynamic loading (fork-and-extend instead)
- Web UI or TUI dashboard (CLI and logs only)

## Module Architecture

```
edgelord/
├── Cargo.toml
├── src/
│   ├── main.rs                   # CLI entry point only
│   ├── lib.rs                    # Public exports for fork-ers
│   │
│   ├── domain/                   # Pure types, zero dependencies
│   │   ├── mod.rs
│   │   ├── market.rs             # Market, Outcome, TokenId
│   │   ├── order_book.rs         # OrderBook, PriceLevel
│   │   ├── position.rs           # Position, Exposure
│   │   ├── opportunity.rs        # Opportunity, Edge, Legs
│   │   ├── relation.rs           # MarketRelation, Cluster
│   │   └── error.rs              # Domain errors (not I/O)
│   │
│   ├── ports/                    # Trait definitions (hexagonal ports)
│   │   ├── mod.rs
│   │   ├── exchange.rs           # MarketDataStream, OrderExecutor
│   │   ├── strategy.rs           # Strategy trait
│   │   ├── notifier.rs           # Notifier trait
│   │   ├── store.rs              # Store trait (persistence)
│   │   ├── solver.rs             # Solver trait (LP/ILP)
│   │   ├── inference.rs          # RelationInferrer trait
│   │   └── risk.rs               # RiskGate trait
│   │
│   ├── adapters/                 # Implementations (hexagonal adapters)
│   │   ├── polymarket/           # Polymarket exchange
│   │   ├── strategies/           # Bundled strategy impls
│   │   ├── notifiers/            # Telegram, logging
│   │   ├── stores/               # SQLite, in-memory
│   │   ├── solvers/              # HiGHS
│   │   └── llm/                  # Anthropic, OpenAI
│   │
│   ├── runtime/                  # Orchestration, wiring
│   │   ├── mod.rs
│   │   ├── orchestrator.rs       # Main event loop
│   │   ├── config.rs             # Configuration loading
│   │   ├── builder.rs            # AppBuilder for composition
│   │   └── governor.rs           # Adaptive scaling
│   │
│   └── cli/                      # Command implementations
│       ├── mod.rs
│       ├── run.rs
│       ├── init.rs               # Guided onboarding
│       ├── status.rs
│       ├── wallet.rs
│       ├── check.rs
│       ├── statistics.rs
│       ├── config.rs
│       ├── strategies.rs         # list, explain commands
│       └── output.rs             # Shared styling/formatting
```

**Dependency rules:**
- `domain/` imports nothing
- `ports/` imports only `domain/`
- `adapters/` imports `domain/` and `ports/`
- `runtime/` imports all above
- `cli/` imports `runtime/` only

## CLI Design

### Command Structure

```
edgelord
├── init                          # Guided setup wizard
├── run                           # Main execution loop
├── status                        # Current state snapshot
├── check
│   ├── config                    # Validate configuration
│   ├── connection                # Test exchange connectivity
│   ├── live                      # Pre-deployment validation
│   └── telegram                  # Test notifications
├── wallet
│   ├── status                    # Approval status
│   ├── address                   # Display address
│   ├── approve                   # Token approvals
│   └── sweep                     # Withdraw USDC
├── statistics
│   ├── today                     # Today's P&L
│   ├── week                      # Weekly summary
│   ├── history                   # Full history
│   ├── export                    # CSV export
│   └── prune                     # Clean old records
├── config
│   ├── init                      # Generate config file
│   ├── show                      # Display current config
│   └── validate                  # Check config validity
├── strategies
│   ├── list                      # Available strategies
│   └── explain <name>            # Describe how it works
├── service
│   ├── install                   # Install systemd unit
│   └── uninstall                 # Remove systemd unit
└── logs                          # Tail application logs
```

### Global Flags

```
--config <path>       Config file (default: ./config.toml)
--json                JSON output for scripting
--quiet               Minimal output
--verbose, -v         Increase verbosity (-vv, -vvv)
--color <when>        auto, always, never
```

### Output Styling

Color palette:

| Element | Color | Usage |
|---------|-------|-------|
| Success | Green | Checkmarks, completed actions, positive P&L |
| Error | Red | Failures, negative P&L |
| Warning | Yellow | Caution states, rejections |
| Info/Accent | Cyan | Commands, values of interest |
| Muted | Dim/Gray | Secondary info, timestamps |
| Primary | Bold white | Headers, primary text |

Example output:

```
$ edgelord run --config config.toml

edgelord v0.2.0

  Network    testnet (amoy)
  Wallet     0x1a2b...3c4d
  Strategies single-condition, market-rebalancing

Subscribing to markets...
  ✓ 847 markets fetched
  ✓ 312 markets scored
  ✓ 128 subscriptions active

Listening for opportunities
  12:04:31 opportunity  $0.47 edge on "Will BTC hit 100k?"
  12:04:31 executed     bought 23 YES @ $0.51, 19 NO @ $0.48
  12:04:33 rejected     below min_profit threshold
```

Error formatting (miette-style):

```
error: configuration invalid

  × strategies.single_condition.min_edge must be between 0 and 1
   ╭─[config.toml:24:1]
24 │ min_edge = 1.5
   ·            ───
   ·             ╰── value out of range
   ╰────
  help: min_edge represents a percentage (e.g., 0.05 = 5%)
```

### Libraries

- `clap` with derive for argument parsing
- `owo-colors` for terminal colors
- `miette` for diagnostic errors
- `indicatif` for progress bars and spinners
- `tabled` for tables
- `dialoguer` for interactive prompts

## Configuration

### File Structure

```toml
# edgelord.toml

[network]
environment = "testnet"              # "testnet" | "mainnet"

[strategies]
enabled = ["single-condition", "market-rebalancing"]

[strategies.single-condition]
min_edge = 0.05
min_profit = 0.50

[strategies.market-rebalancing]
min_edge = 0.03

[strategies.combinatorial]
min_edge = 0.02

[risk]
max_exposure = 500.0
max_position_per_market = 100.0
max_slippage = 0.02

[notifications]
backend = "telegram"                 # "telegram" | "none"

[inference]
provider = "anthropic"               # "anthropic" | "openai" | "none"

[database]
path = "~/.local/share/edgelord/edgelord.db"

[governor]
enabled = true
target_latency_p99_ms = 100
```

### Environment Variables

Secrets only:

```bash
EDGELORD_WALLET_KEY=0x...
EDGELORD_TELEGRAM_TOKEN=...
EDGELORD_TELEGRAM_CHAT=...
ANTHROPIC_API_KEY=...
OPENAI_API_KEY=...
```

### Resolution Order

1. CLI flags (highest priority)
2. Environment variables (secrets)
3. Config file
4. Built-in defaults (lowest)

### XDG Paths

- Config: `~/.config/edgelord/config.toml`
- Data: `~/.local/share/edgelord/`
- Logs: `~/.local/state/edgelord/logs/`

## Extension Points

### Public API

```rust
// lib.rs

//! Edgelord: Prediction market arbitrage framework
//!
//! # For CLI users
//! `cargo install edgelord` and run `edgelord --help`
//!
//! # For developers
//! Fork this repo and extend:
//! - Add strategies: implement `ports::Strategy`
//! - Add exchanges: implement `ports::MarketDataStream` + `ports::OrderExecutor`
//! - Add notifiers: implement `ports::Notifier`

pub mod domain;
pub mod ports;
pub mod adapters;
pub mod runtime;
```

### Custom Strategy Example

```rust
use edgelord::domain::{Opportunity, OrderBook, Market};
use edgelord::ports::Strategy;

pub struct MomentumStrategy {
    lookback_window: usize,
}

impl Strategy for MomentumStrategy {
    fn name(&self) -> &str {
        "momentum"
    }

    fn detect(
        &self,
        market: &Market,
        book: &OrderBook,
        ctx: &StrategyContext,
    ) -> Option<Opportunity> {
        // Detection logic here
    }
}
```

### Custom Orchestrator

```rust
use edgelord::runtime::Builder;
use my_strategies::MomentumStrategy;

fn main() {
    let app = Builder::new()
        .config_path("config.toml")
        .strategy(MomentumStrategy::new(20))
        .strategy(SingleCondition::default())
        .build()
        .unwrap();

    app.run();
}
```

## Documentation

### Structure

```
docs/
├── getting-started.md
├── configuration.md
├── strategies/
│   ├── overview.md
│   ├── single-condition.md
│   ├── market-rebalancing.md
│   ├── combinatorial.md
│   └── custom.md
├── extending/
│   ├── strategies.md
│   ├── exchanges.md
│   ├── notifiers.md
│   └── architecture.md
└── deployment/
    ├── systemd.md
    └── operations.md
```

### Writing Style

- Direct and concise. No filler words.
- Active voice. "Run this" not "This can be run".
- Real examples. Every concept has working code.
- No AI tells. No em-dashes, no "straightforward", no "leverage".
- Consistent terminology throughout.

## Implementation Plan

### Phase 1: Restructure

- Create new module structure
- Move existing code into new locations
- Preserve all functionality
- Ensure tests pass

### Phase 2: Clean Interfaces

- Extract traits into `ports/`
- Rename to match conventions
- Document every public type
- Enforce dependency rules

### Phase 3: CLI Overhaul

- Add `cli/output.rs` with formatting
- Integrate `miette` for errors
- Build `edgelord init` wizard
- Add `edgelord strategies` commands
- Consistent flags across commands

### Phase 4: Documentation

- Rewrite README
- Update all docs to new style
- Add extending guides
- Doc comments on all public items

### Phase 5: Polish and Release

- Audit public API surface
- Integration tests for CLI output
- Test `cargo install` flow
- Publish to crates.io

## What Stays Unchanged

- Core algorithms (strategies, risk, governor)
- Polymarket client implementation
- Database schema and migrations
- Telegram integration
- LLM inference logic
