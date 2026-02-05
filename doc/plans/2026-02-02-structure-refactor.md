# Structure Refactor Implementation Plan

> Status: Historical
> Superseded by: N/A
> Summary:
> - Goal: Reorganize src/ into a cleaner hierarchy with `core/` containing all library code and `app/` containing application orchestration.
> - Scope: Task 1.1: Create core/domain/ (move pure types)
> Planned Outcomes:
> - Task 1.1: Create core/domain/ (move pure types)
> - Task 1.2: Move strategy/ to core/


> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Reorganize src/ into a cleaner hierarchy with `core/` containing all library code and `app/` containing application orchestration.

**Architecture:** Create `core/` as the container for all reusable library code. Move domain types to `core/domain/`, exchange implementations to `core/exchange/polymarket/`, and promote strategy/solver/service to `core/`. Keep `app/` at top level for application wiring. Update all imports throughout the codebase.

**Tech Stack:** Rust 2021, standard module system

---

## Target Structure

```
src/
├── core/                      # All reusable library code
│   ├── mod.rs
│   ├── domain/                # Pure types (was src/domain/ minus strategy/solver)
│   │   ├── mod.rs
│   │   ├── id.rs
│   │   ├── money.rs
│   │   ├── market.rs
│   │   ├── orderbook.rs
│   │   ├── opportunity.rs
│   │   └── position.rs
│   ├── exchange/              # Exchange traits + implementations
│   │   ├── mod.rs             # Traits (was src/exchange/traits.rs content)
│   │   ├── factory.rs         # (was src/exchange/factory.rs)
│   │   └── polymarket/        # (was src/adapter/polymarket/)
│   ├── strategy/              # Detection algorithms (was src/domain/strategy/)
│   ├── solver/                # LP/ILP (was src/domain/solver/)
│   └── service/               # Cross-cutting (was src/service/)
│
├── app/                       # Application orchestration (unchanged location)
│   ├── mod.rs
│   ├── config.rs
│   ├── orchestrator.rs
│   └── state.rs
│
├── lib.rs
├── main.rs
└── error.rs
```

---

## Part 1: Create core/ Structure

### Task 1.1: Create core/domain/ (move pure types)

**Files:**
- Create: `src/core/mod.rs`
- Create: `src/core/domain/mod.rs`
- Move: `src/domain/id.rs` → `src/core/domain/id.rs`
- Move: `src/domain/money.rs` → `src/core/domain/money.rs`
- Move: `src/domain/market.rs` → `src/core/domain/market.rs`
- Move: `src/domain/orderbook.rs` → `src/core/domain/orderbook.rs`
- Move: `src/domain/opportunity.rs` → `src/core/domain/opportunity.rs`
- Move: `src/domain/position.rs` → `src/core/domain/position.rs`

**Step 1: Create directory and move files**

```bash
mkdir -p src/core/domain
mv src/domain/id.rs src/core/domain/
mv src/domain/money.rs src/core/domain/
mv src/domain/market.rs src/core/domain/
mv src/domain/orderbook.rs src/core/domain/
mv src/domain/opportunity.rs src/core/domain/
mv src/domain/position.rs src/core/domain/
```

**Step 2: Create src/core/domain/mod.rs**

```rust
//! Pure domain types.

mod id;
mod market;
mod money;
mod opportunity;
mod orderbook;
mod position;

pub use id::{MarketId, TokenId};
pub use market::MarketPair;
pub use money::{Price, Volume};
pub use opportunity::{Opportunity, OpportunityBuildError, OpportunityBuilder};
pub use orderbook::{OrderBook, OrderBookCache, PriceLevel};
pub use position::{Position, PositionId, PositionLeg, PositionStatus, PositionTracker};
```

**Step 3: Create src/core/mod.rs (initial)**

```rust
//! Core library components.

pub mod domain;
```

**Step 4: Update internal imports in moved files**

In each moved file, update `use super::` and `use crate::domain::` references:
- `super::id` → `super::id` (unchanged, relative)
- `super::money` → `super::money` (unchanged, relative)
- `crate::domain::X` → `crate::core::domain::X`

**Step 5: Don't compile yet** - continue to next task

---

### Task 1.2: Move strategy/ to core/

**Files:**
- Move: `src/domain/strategy/` → `src/core/strategy/`

**Step 1: Move directory**

```bash
mv src/domain/strategy src/core/
```

**Step 2: Update src/core/mod.rs**

```rust
//! Core library components.

pub mod domain;
pub mod strategy;
```

**Step 3: Update imports in strategy files**

In `src/core/strategy/*.rs`, update:
- `crate::domain::X` → `crate::core::domain::X`

---

### Task 1.3: Move solver/ to core/

**Files:**
- Move: `src/domain/solver/` → `src/core/solver/`

**Step 1: Move directory**

```bash
mv src/domain/solver src/core/
```

**Step 2: Update src/core/mod.rs**

```rust
//! Core library components.

pub mod domain;
pub mod solver;
pub mod strategy;
```

---

### Task 1.4: Move service/ to core/

**Files:**
- Move: `src/service/` → `src/core/service/`

**Step 1: Move directory**

```bash
mv src/service src/core/
```

**Step 2: Update src/core/mod.rs**

```rust
//! Core library components.

pub mod domain;
pub mod service;
pub mod solver;
pub mod strategy;
```

**Step 3: Update imports in service files**

In `src/core/service/*.rs`, update:
- `crate::domain::X` → `crate::core::domain::X`
- `crate::error::X` → `crate::error::X` (unchanged)

---

### Task 1.5: Move exchange/ to core/ and merge with adapter/

**Files:**
- Move: `src/exchange/traits.rs` content → `src/core/exchange/mod.rs`
- Move: `src/exchange/factory.rs` → `src/core/exchange/factory.rs`
- Move: `src/adapter/polymarket/` → `src/core/exchange/polymarket/`
- Delete: `src/exchange/` (old)
- Delete: `src/adapter/` (old)

**Step 1: Create core/exchange and move files**

```bash
mkdir -p src/core/exchange
mv src/adapter/polymarket src/core/exchange/
mv src/exchange/factory.rs src/core/exchange/
```

**Step 2: Create src/core/exchange/mod.rs**

Take content from `src/exchange/traits.rs` and `src/exchange/mod.rs`, combine:

```rust
//! Exchange abstractions and implementations.

mod factory;
pub mod polymarket;

// Re-export factory
pub use factory::ExchangeFactory;

// === Trait definitions (from traits.rs) ===

use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::core::domain::{OrderBook, TokenId};
use crate::error::Error;

// ... rest of traits.rs content with updated imports
```

**Step 3: Update imports in polymarket/ files**

In `src/core/exchange/polymarket/*.rs`, update:
- `crate::domain::X` → `crate::core::domain::X`
- `crate::exchange::X` → `crate::core::exchange::X`
- `crate::app::X` → `crate::app::X` (unchanged)

**Step 4: Update factory.rs imports**

```rust
use crate::app::{Config, Exchange};
use crate::error::Result;

use super::{MarketDataStream, MarketFetcher, OrderExecutor};
```

**Step 5: Delete old directories**

```bash
rm -rf src/exchange
rm -rf src/adapter
rm src/domain/mod.rs
rmdir src/domain
```

**Step 6: Update src/core/mod.rs (final)**

```rust
//! Core library components.

pub mod domain;
pub mod exchange;
pub mod service;
pub mod solver;
pub mod strategy;
```

---

## Part 2: Update Application Layer

### Task 2.1: Update app/ imports

**Files:**
- Modify: `src/app/orchestrator.rs`
- Modify: `src/app/state.rs`
- Modify: `src/app/config.rs`

**Step 1: Update orchestrator.rs imports**

Replace:
- `crate::adapter::polymarket::` → `crate::core::exchange::polymarket::`
- `crate::domain::` → `crate::core::domain::`
- `crate::exchange::` → `crate::core::exchange::`
- `crate::service::` → `crate::core::service::`

**Step 2: Update state.rs imports**

Replace:
- `crate::domain::` → `crate::core::domain::`

**Step 3: Update config.rs imports**

Replace:
- `crate::domain::strategy::` → `crate::core::strategy::`

---

### Task 2.2: Update lib.rs

**Files:**
- Modify: `src/lib.rs`

**New content:**

```rust
//! Edgelord - Multi-strategy arbitrage detection and execution.
//!
//! # Architecture
//!
//! ```text
//! src/
//! ├── core/             # Reusable library components
//! │   ├── domain/       # Pure domain types
//! │   ├── exchange/     # Exchange traits + implementations
//! │   ├── strategy/     # Detection algorithms
//! │   ├── solver/       # LP/ILP solver abstraction
//! │   └── service/      # Cross-cutting services
//! └── app/              # Application orchestration
//! ```

pub mod core;
pub mod error;
pub mod app;
```

---

### Task 2.3: Update error.rs imports

**Files:**
- Modify: `src/error.rs`

Check if error.rs has any imports from moved modules and update them.

---

## Part 3: Fix All Remaining Imports

### Task 3.1: Grep and fix remaining old imports

**Step 1: Find all remaining old imports**

```bash
grep -r "crate::domain::" src/
grep -r "crate::exchange::" src/
grep -r "crate::adapter::" src/
grep -r "crate::service::" src/
```

**Step 2: Fix each occurrence**

- `crate::domain::` → `crate::core::domain::`
- `crate::domain::strategy::` → `crate::core::strategy::`
- `crate::domain::solver::` → `crate::core::solver::`
- `crate::exchange::` → `crate::core::exchange::`
- `crate::adapter::` → `crate::core::exchange::`
- `crate::service::` → `crate::core::service::`

---

## Part 4: Verification

### Task 4.1: Build and test

**Step 1: Build**

```bash
cargo build --all-features
```

Fix any compilation errors.

**Step 2: Run tests**

```bash
cargo test --all-features
```

All 114+ tests should pass.

**Step 3: Run clippy**

```bash
cargo clippy --all-features
```

Should have 0 warnings.

**Step 4: Commit**

```bash
git add -A
git commit -m "refactor: reorganize src/ into core/ and app/ hierarchy"
```

---

### Task 4.2: Update documentation

**Files:**
- Modify: `doc/architecture/system-design.md`

Update the module structure section to reflect new hierarchy.

**Step 1: Commit docs**

```bash
git add -A
git commit -m "docs: update architecture for new structure"
```

---

## Summary

| Task | Description |
|------|-------------|
| 1.1 | Create core/domain/ with pure types |
| 1.2 | Move strategy/ to core/ |
| 1.3 | Move solver/ to core/ |
| 1.4 | Move service/ to core/ |
| 1.5 | Move exchange/ + adapter/polymarket/ to core/exchange/ |
| 2.1 | Update app/ imports |
| 2.2 | Update lib.rs |
| 2.3 | Update error.rs imports |
| 3.1 | Fix all remaining imports |
| 4.1 | Build, test, clippy |
| 4.2 | Update documentation |
