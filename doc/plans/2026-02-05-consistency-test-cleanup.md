# Consistency + Test Cleanup Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Align architecture, naming, layering, and tests with project standards while consolidating reusable test helpers.

**Architecture:** Move Frank-Wolfe + Bregman into `core::solver`, add an `app` façade for CLI stats/status, rename plural modules to singular or documented exceptions, and split `statistics` so it stays under the SLOC limit. Add a structured `tests/support` helper layout and normalize integration test names.

**Tech Stack:** Rust, Diesel/SQLite, tokio, rust_decimal

---

### Task 1: Update Architecture Doc (Orchestrator Folder, SLOC Rule, Dependencies, Exceptions)

**Files:**
- Modify: `ARCHITECTURE.md`

**Step 1: Add doc changes that should be visible in a diff**

Update these sections in `ARCHITECTURE.md`:
- **File Size Limits**: clarify SLOC (tests excluded)
- **Directory Structure**: change `app/orchestrator.rs` to `app/orchestrator/`
- **Dependency Rules**: allow `strategy -> solver`
- **Naming Exceptions**: allow `types.rs` due to Rust keyword `type`

Suggested patch (adjust wording if needed):

```markdown
## File Size Limits

- **Hard limit: 400 SLOC (source lines of code)**
- Tests (including `#[cfg(test)]` modules) **do not count** toward the limit
- Approaching limit? Split into submodule
- Prefer many small files over few large ones
- Exception: Generated code, test fixtures
```

```markdown
## Dependency Rules

cli → app → {exchange, strategy, service} → domain
strategy → solver
```

```markdown
## Directory Structure

├── app/                        # Application layer
│   ├── mod.rs
│   ├── orchestrator/           # Main event loop
│   │   ├── mod.rs
│   │   ├── handler.rs
│   │   └── execution.rs
```

```markdown
## Naming Conventions

### Exceptions
- `types.rs` is allowed where `type.rs` would conflict with Rust keywords.
```

**Step 2: Run a quick search to ensure doc paths match code**

Run:
```bash
rg "orchestrator" ARCHITECTURE.md
```
Expected: only folder references remain.

**Step 3: Commit**

```bash
git add ARCHITECTURE.md
git commit -m "docs: align architecture rules with current structure"
```

---

### Task 2: Move Frank-Wolfe + Bregman Into `core::solver`

**Files:**
- Create: `src/core/solver/frank_wolfe.rs`
- Create: `src/core/solver/bregman.rs`
- Modify: `src/core/solver/mod.rs`
- Modify: `src/core/strategy/combinatorial/mod.rs`
- Modify: `src/core/service/cluster/detector.rs`
- Delete: `src/core/strategy/combinatorial/frank_wolfe.rs`
- Delete: `src/core/strategy/combinatorial/bregman.rs`

**Step 1: Introduce a failing compile change**

In `src/core/service/cluster/detector.rs`, change the import to:

```rust
use crate::core::solver::{FrankWolfe, FrankWolfeConfig};
```

**Step 2: Run a targeted compile to confirm failure**

Run:
```bash
cargo test -p edgelord core::service::cluster::tests::test_detector_gap_below_threshold
```
Expected: compile error that `FrankWolfe` is not found in `core::solver`.

**Step 3: Move code into solver modules**

- Copy `src/core/strategy/combinatorial/bregman.rs` to `src/core/solver/bregman.rs` and update the module docs to say "Solver utilities for LMSR Bregman divergence" (no API changes required).
- Copy `src/core/strategy/combinatorial/frank_wolfe.rs` to `src/core/solver/frank_wolfe.rs` and update imports to:

```rust
use crate::core::solver::{IlpProblem, LpProblem, SolutionStatus, Solver};
use crate::core::solver::bregman::{bregman_divergence, bregman_gradient};
use crate::error::Result;
```

- In `src/core/solver/mod.rs`, add:

```rust
mod bregman;
mod frank_wolfe;

pub use bregman::{bregman_divergence, bregman_gradient, lmsr_cost, lmsr_prices};
pub use frank_wolfe::{FrankWolfe, FrankWolfeConfig, FrankWolfeResult};
```

- In `src/core/strategy/combinatorial/mod.rs`, remove the `mod bregman; mod frank_wolfe;` entries and re-export from solver if needed:

```rust
pub use crate::core::solver::{FrankWolfe, FrankWolfeConfig, FrankWolfeResult};
```

**Step 4: Run tests again**

Run:
```bash
cargo test -p edgelord core::service::cluster::tests::test_detector_gap_below_threshold
```
Expected: PASS.

**Step 5: Commit**

```bash
git add src/core/solver src/core/strategy/combinatorial src/core/service/cluster/detector.rs
git commit -m "refactor: move frank-wolfe and bregman to solver"
```

---

### Task 3: Rename Polymarket `messages.rs` → `message.rs`

**Files:**
- Modify: `src/core/exchange/polymarket/mod.rs`
- Modify: `src/core/exchange/polymarket/websocket.rs`
- Modify: `src/core/exchange/polymarket/client.rs` (if importing messages)
- Rename: `src/core/exchange/polymarket/messages.rs` → `src/core/exchange/polymarket/message.rs`

**Step 1: Create failing import by renaming module reference**

In `src/core/exchange/polymarket/mod.rs`, update to:

```rust
mod message;

pub use message::{PolymarketBookMessage, PolymarketWsMessage, PolymarketWsPriceLevel};
```

**Step 2: Run a compile check**

Run:
```bash
cargo test -p edgelord core::exchange::polymarket::tests::test_polymarket_ws_parse
```
Expected: compile failure until the file is renamed and imports updated.

**Step 3: Rename file and fix imports**

```bash
git mv src/core/exchange/polymarket/messages.rs src/core/exchange/polymarket/message.rs
```
Update any imports from `messages::` to `message::` in `client.rs` or `websocket.rs`.

**Step 4: Run the targeted test**

```bash
cargo test -p edgelord core::exchange::polymarket::tests::test_polymarket_ws_parse
```
Expected: PASS.

**Step 5: Commit**

```bash
git add src/core/exchange/polymarket
git commit -m "refactor: rename polymarket messages module"
```

---

### Task 4: Rename `stats` → `statistics` and Split to <400 SLOC

**Files:**
- Create: `src/core/service/statistics/mod.rs`
- Create: `src/core/service/statistics/types.rs`
- Create: `src/core/service/statistics/convert.rs`
- Create: `src/core/service/statistics/recorder.rs`
- Modify: `src/core/service/mod.rs`
- Delete: `src/core/service/stats/mod.rs`
- Modify: all imports from `core::service::stats` → `core::service::statistics`

**Step 1: Introduce a failing import**

In `src/core/service/mod.rs`, replace:
```rust
pub mod stats;
```
with:
```rust
pub mod statistics;
```

**Step 2: Run a compile check**

Run:
```bash
cargo test -p edgelord core::service::stats::tests::stats_summary_win_rate_with_trades
```
Expected: compile failure (module `stats` missing).

**Step 3: Create new `statistics` module files**

Create `src/core/service/statistics/types.rs`:

```rust
use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct RecordedOpportunity {
    pub strategy: String,
    pub market_ids: Vec<String>,
    pub edge: Decimal,
    pub expected_profit: Decimal,
    pub executed: bool,
    pub rejected_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TradeOpenEvent {
    pub opportunity_id: i32,
    pub strategy: String,
    pub market_ids: Vec<String>,
    pub legs: Vec<TradeLeg>,
    pub size: Decimal,
    pub expected_profit: Decimal,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TradeLeg {
    pub token_id: String,
    pub side: String,
    pub price: Decimal,
    pub size: Decimal,
}

#[derive(Debug, Clone)]
pub struct TradeCloseEvent {
    pub trade_id: i32,
    pub realized_profit: Decimal,
    pub reason: String,
}

#[derive(Debug, Clone, Default)]
pub struct StatsSummary {
    pub opportunities_detected: i64,
    pub opportunities_executed: i64,
    pub opportunities_rejected: i64,
    pub trades_opened: i64,
    pub trades_closed: i64,
    pub profit_realized: Decimal,
    pub loss_realized: Decimal,
    pub win_count: i64,
    pub loss_count: i64,
    pub total_volume: Decimal,
}

impl StatsSummary {
    #[must_use]
    pub fn win_rate(&self) -> Option<f64> {
        let total = self.win_count + self.loss_count;
        if total == 0 {
            None
        } else {
            Some(self.win_count as f64 / total as f64 * 100.0)
        }
    }

    #[must_use]
    pub fn net_profit(&self) -> Decimal {
        self.profit_realized - self.loss_realized
    }
}

#[derive(Debug, Clone)]
pub struct OpportunitySummary {
    pub id: i32,
    pub strategy: String,
    pub edge: Decimal,
    pub expected_profit: Decimal,
    pub executed: bool,
    pub rejected_reason: Option<String>,
    pub detected_at: String,
}
```

Create `src/core/service/statistics/convert.rs`:

```rust
use rust_decimal::Decimal;

pub fn decimal_to_f32(d: Decimal) -> f32 {
    use rust_decimal::prelude::ToPrimitive;
    d.to_f32().unwrap_or(0.0)
}

pub fn f32_to_decimal(f: f32) -> Decimal {
    use rust_decimal::prelude::FromPrimitive;
    Decimal::from_f32(f).unwrap_or(Decimal::ZERO)
}
```

Create `src/core/service/statistics/recorder.rs` by moving the `StatsRecorder` struct and all impl methods from the old `stats/mod.rs` (update imports to pull types and helpers from `types` and `convert`). Keep the existing logic unchanged.

Create `src/core/service/statistics/mod.rs`:

```rust
mod convert;
mod recorder;
mod types;

pub use convert::{decimal_to_f32, f32_to_decimal};
pub use recorder::{create_recorder, StatsRecorder};
pub use types::{
    OpportunitySummary,
    RecordedOpportunity,
    StatsSummary,
    TradeCloseEvent,
    TradeLeg,
    TradeOpenEvent,
};
```

Move the `#[cfg(test)]` tests from the old file into `statistics/mod.rs` or a new `statistics/tests.rs` and update imports accordingly.

**Step 4: Update imports across the codebase**

Use `rg "service::stats" -l` and replace with `service::statistics`.

**Step 5: Run tests and commit**

```bash
cargo test -p edgelord core::service::statistics::tests::stats_summary_win_rate_with_trades
```
Expected: PASS.

```bash
git add src/core/service src/app src/cli
git commit -m "refactor: rename stats service and split module"
```

---

### Task 5: Add `app` Facades for Stats/Status to Fix CLI Layering

**Files:**
- Create: `src/app/stats.rs`
- Create: `src/app/status.rs`
- Modify: `src/app/mod.rs`
- Modify: `src/cli/stats.rs`
- Modify: `src/cli/status.rs`

**Step 1: Create compile failure by switching CLI to app**

In `src/cli/stats.rs` and `src/cli/status.rs`, replace direct `core::db` imports with:

```rust
use crate::app;
```

Then update calls to use `app::stats::...` and `app::status::...` (functions to be added next).

**Step 2: Run a compile check**

```bash
cargo test -p edgelord cli_tests::cli_returns_nonzero_on_config_error
```
Expected: compile failure until the `app` façade exists.

**Step 3: Implement app façade modules**

Create `src/app/stats.rs` with explicit data access helpers (example skeleton):

```rust
use std::path::Path;

use chrono::{Duration, NaiveDate, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};

use crate::core::db::model::{DailyStatsRow, StrategyDailyStatsRow};
use crate::core::db::schema::{daily_stats, strategy_daily_stats, trades};
use crate::core::service::statistics::StatsSummary;
use crate::error::{ConfigError, Error, Result};

fn connect(db_path: &Path) -> Result<Pool<ConnectionManager<SqliteConnection>>> {
    let db_url = format!("sqlite://{}", db_path.display());
    let manager = ConnectionManager::<SqliteConnection>::new(&db_url);
    Pool::builder()
        .max_size(1)
        .build(manager)
        .map_err(|e| Error::Config(ConfigError::Other(e.to_string())))
}

pub fn load_summary(db_path: &Path, from: NaiveDate, to: NaiveDate) -> Result<StatsSummary> {
    let pool = connect(db_path)?;
    let mut conn = pool.get().map_err(|e| Error::Config(ConfigError::Other(e.to_string())))?;
    let rows: Vec<DailyStatsRow> = daily_stats::table
        .filter(daily_stats::date.ge(from.to_string()))
        .filter(daily_stats::date.le(to.to_string()))
        .load(&mut conn)
        .unwrap_or_default();

    let mut summary = StatsSummary::default();
    for row in rows {
        summary.opportunities_detected += i64::from(row.opportunities_detected);
        summary.opportunities_executed += i64::from(row.opportunities_executed);
        summary.opportunities_rejected += i64::from(row.opportunities_rejected);
        summary.trades_opened += i64::from(row.trades_opened);
        summary.trades_closed += i64::from(row.trades_closed);
        summary.profit_realized += crate::core::service::statistics::f32_to_decimal(row.profit_realized);
        summary.loss_realized += crate::core::service::statistics::f32_to_decimal(row.loss_realized);
        summary.win_count += i64::from(row.win_count);
        summary.loss_count += i64::from(row.loss_count);
        summary.total_volume += crate::core::service::statistics::f32_to_decimal(row.total_volume);
    }

    Ok(summary)
}

pub fn load_strategy_breakdown(
    db_path: &Path,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<StrategyDailyStatsRow>> {
    let pool = connect(db_path)?;
    let mut conn = pool.get().map_err(|e| Error::Config(ConfigError::Other(e.to_string())))?;
    let rows: Vec<StrategyDailyStatsRow> = strategy_daily_stats::table
        .filter(strategy_daily_stats::date.ge(from.to_string()))
        .filter(strategy_daily_stats::date.le(to.to_string()))
        .load(&mut conn)
        .unwrap_or_default();
    Ok(rows)
}

pub fn load_open_positions(db_path: &Path) -> Result<i64> {
    let pool = connect(db_path)?;
    let mut conn = pool.get().map_err(|e| Error::Config(ConfigError::Other(e.to_string())))?;
    let open_count: i64 = trades::table
        .filter(trades::status.eq("open"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);
    Ok(open_count)
}

pub fn date_range_today() -> (NaiveDate, NaiveDate, String) {
    let today = Utc::now().date_naive();
    (today, today, "Today".to_string())
}

pub fn date_range_week() -> (NaiveDate, NaiveDate, String) {
    let today = Utc::now().date_naive();
    let week_ago = today - Duration::days(7);
    (week_ago, today, "Last 7 Days".to_string())
}

pub fn date_range_history(days: u32) -> (NaiveDate, NaiveDate, String) {
    let today = Utc::now().date_naive();
    let start = today - Duration::days(i64::from(days));
    (start, today, format!("Last {days} Days"))
}
```

Create `src/app/status.rs` (summary data for CLI status):

```rust
use std::path::Path;

use chrono::{Duration, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};

use crate::core::db::model::DailyStatsRow;
use crate::core::db::schema::{daily_stats, trades};
use crate::error::{ConfigError, Error, Result};

fn connect(db_path: &Path) -> Result<Pool<ConnectionManager<SqliteConnection>>> {
    let db_url = format!("sqlite://{}", db_path.display());
    let manager = ConnectionManager::<SqliteConnection>::new(&db_url);
    Pool::builder()
        .max_size(1)
        .build(manager)
        .map_err(|e| Error::Config(ConfigError::Other(e.to_string())))
}

pub struct StatusSnapshot {
    pub today: Option<DailyStatsRow>,
    pub week_rows: Vec<DailyStatsRow>,
    pub open_positions: i64,
}

pub fn load_status(db_path: &Path) -> Result<StatusSnapshot> {
    let pool = connect(db_path)?;
    let mut conn = pool.get().map_err(|e| Error::Config(ConfigError::Other(e.to_string())))?;

    let today = Utc::now().date_naive();
    let week_ago = today - Duration::days(7);

    let today_row: Option<DailyStatsRow> = daily_stats::table
        .filter(daily_stats::date.eq(today.to_string()))
        .first(&mut conn)
        .ok();

    let week_rows: Vec<DailyStatsRow> = daily_stats::table
        .filter(daily_stats::date.ge(week_ago.to_string()))
        .filter(daily_stats::date.le(today.to_string()))
        .load(&mut conn)
        .unwrap_or_default();

    let open_positions: i64 = trades::table
        .filter(trades::status.eq("open"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    Ok(StatusSnapshot {
        today: today_row,
        week_rows,
        open_positions,
    })
}
```

Update `src/app/mod.rs`:

```rust
pub mod stats;
pub mod status;
```

**Step 4: Update CLI to use app façade**

- In `src/cli/stats.rs`, replace DB access with calls to `app::stats::{load_summary, load_strategy_breakdown, load_open_positions, date_range_*}`.
- In `src/cli/status.rs`, call `app::status::load_status` and use returned `StatusSnapshot` to display metrics.

**Step 5: Run tests and commit**

```bash
cargo test -p edgelord cli_tests::cli_returns_nonzero_on_config_error
```
Expected: PASS.

```bash
git add src/app src/cli
git commit -m "refactor: add app facades for stats and status"
```

---

### Task 6: Add `tests/support` Helpers

**Files:**
- Create: `tests/support/mod.rs`
- Create: `tests/support/market.rs`
- Create: `tests/support/order_book.rs`
- Create: `tests/support/registry.rs`
- Create: `tests/support/relation.rs`
- Create: `tests/support/config.rs`
- Create: `tests/support/assertions.rs`

**Step 1: Update a test to reference new helpers (intentional failure)**

In `tests/strategy_tests.rs`, replace the local `make_market()` with `support::market::make_binary_market`, which does not exist yet.

**Step 2: Run the test to confirm failure**

```bash
cargo test --test strategy_tests
```
Expected: compile error missing `tests::support`.

**Step 3: Create helper modules**

Create `tests/support/mod.rs`:

```rust
pub mod assertions;
pub mod config;
pub mod market;
pub mod order_book;
pub mod registry;
pub mod relation;
```

Create `tests/support/market.rs`:

```rust
use rust_decimal::Decimal;

use edgelord::core::domain::{Market, MarketId, Outcome, TokenId};

pub fn make_binary_market(
    id: &str,
    question: &str,
    yes_token: &str,
    no_token: &str,
    payout: Decimal,
) -> Market {
    let outcomes = vec![
        Outcome::new(TokenId::from(yes_token), "Yes"),
        Outcome::new(TokenId::from(no_token), "No"),
    ];
    Market::new(MarketId::from(id), question, outcomes, payout)
}

pub fn make_multi_market(
    id: &str,
    question: &str,
    outcomes: &[(&str, &str)],
    payout: Decimal,
) -> Market {
    let outcomes = outcomes
        .iter()
        .map(|(token, name)| Outcome::new(TokenId::from(*token), *name))
        .collect();
    Market::new(MarketId::from(id), question, outcomes, payout)
}
```

Create `tests/support/order_book.rs`:

```rust
use rust_decimal::Decimal;

use edgelord::core::cache::OrderBookCache;
use edgelord::core::domain::{OrderBook, PriceLevel, TokenId};

pub fn make_order_book(token_id: &str, bid: Decimal, ask: Decimal) -> OrderBook {
    OrderBook::with_levels(
        TokenId::from(token_id),
        vec![PriceLevel::new(bid, Decimal::new(100, 0))],
        vec![PriceLevel::new(ask, Decimal::new(100, 0))],
    )
}

pub fn set_order_book(cache: &OrderBookCache, token_id: &str, bid: Decimal, ask: Decimal) {
    cache.update(make_order_book(token_id, bid, ask));
}
```

Create `tests/support/registry.rs`:

```rust
use edgelord::core::domain::{Market, MarketRegistry};

pub fn make_registry(markets: Vec<Market>) -> MarketRegistry {
    let mut registry = MarketRegistry::new();
    for market in markets {
        registry.add(market);
    }
    registry
}
```

Create `tests/support/relation.rs`:

```rust
use edgelord::core::domain::{MarketId, Relation, RelationKind};

pub fn mutually_exclusive(markets: &[&str], confidence: f64, reasoning: &str) -> Relation {
    Relation::new(
        RelationKind::MutuallyExclusive {
            markets: markets.iter().map(|m| MarketId::from(*m)).collect(),
        },
        confidence,
        reasoning,
    )
}

pub fn exactly_one(markets: &[&str], confidence: f64, reasoning: &str) -> Relation {
    Relation::new(
        RelationKind::ExactlyOne {
            markets: markets.iter().map(|m| MarketId::from(*m)).collect(),
        },
        confidence,
        reasoning,
    )
}
```

Create `tests/support/config.rs`:

```rust
use edgelord::app::ReconnectionConfig;

pub fn test_reconnection_config() -> ReconnectionConfig {
    ReconnectionConfig {
        initial_delay_ms: 0,
        max_delay_ms: 0,
        backoff_multiplier: 1.0,
        max_consecutive_failures: 3,
        circuit_breaker_cooldown_ms: 0,
    }
}
```

Create `tests/support/assertions.rs`:

```rust
use rust_decimal::Decimal;

pub fn assert_decimal_near(actual: Decimal, expected: Decimal, tolerance: Decimal) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= tolerance,
        "expected {} ± {}, got {}",
        expected,
        tolerance,
        actual
    );
}
```

**Step 4: Re-run the test**

```bash
cargo test --test strategy_tests
```
Expected: PASS.

**Step 5: Commit**

```bash
git add tests/support tests/strategy_tests.rs
git commit -m "test: add shared support helpers"
```

---

### Task 7: Migrate Integration Tests to Helpers

**Files:**
- Modify: `tests/strategy_tests.rs`
- Modify: `tests/cluster_detection_tests.rs`
- Modify: `tests/exchange_tests.rs`
- Modify: `tests/multi_exchange_abstraction.rs`

**Step 1: Update one test file to use helpers (compile failure)**

Start with `tests/cluster_detection_tests.rs`:
- Replace `create_test_market`, `create_order_book`, and `setup_test_environment` with helper calls.

**Step 2: Run the updated test**

```bash
cargo test --test cluster_detection_tests
```
Expected: compile failure until helpers are wired correctly.

**Step 3: Apply helper refactors**

Examples:
- Use `support::market::make_binary_market` for markets.
- Use `support::order_book::{make_order_book, set_order_book}` for books.
- Use `support::registry::make_registry` for `MarketRegistry`.
- Use `support::relation::mutually_exclusive` / `exactly_one` for relations.

Repeat for `tests/strategy_tests.rs`, `tests/exchange_tests.rs`, `tests/multi_exchange_abstraction.rs`.

**Step 4: Run the affected tests**

```bash
cargo test --test cluster_detection_tests
cargo test --test strategy_tests
cargo test --test exchange_tests
cargo test --test multi_exchange_abstraction
```
Expected: PASS.

**Step 5: Commit**

```bash
git add tests/*.rs
git commit -m "test: migrate integration tests to support helpers"
```

---

### Task 8: Normalize Integration Test File Names

**Files:**
- Rename: `tests/inference_integration.rs` → `tests/inference_tests.rs`
- Rename: `tests/multi_exchange_abstraction.rs` → `tests/multi_exchange_tests.rs`

**Step 1: Rename and update references**

```bash
git mv tests/inference_integration.rs tests/inference_tests.rs
git mv tests/multi_exchange_abstraction.rs tests/multi_exchange_tests.rs
```

**Step 2: Run tests**

```bash
cargo test --test inference_tests
cargo test --test multi_exchange_tests
```
Expected: PASS.

**Step 3: Commit**

```bash
git add tests
git commit -m "test: normalize integration test names"
```

---

### Task 9: Full Test Pass

**Files:**
- Test: `cargo test`

**Step 1: Run full suite**

```bash
cargo test
```
Expected: PASS.

**Step 2: Commit (if any final fixes were required)**

```bash
git add -A
git commit -m "chore: final consistency cleanup"
```

---

## Notes
- If `cargo test` fails because crates cannot be downloaded, retry after ensuring network access or use a pre-populated cargo registry.
- `types.rs` is a documented naming exception because `type.rs` is invalid in Rust.
