# Open-Source Framework Refactor

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Restructure edgelord into a CLI-first open-source framework with hexagonal architecture (domain/ports/adapters/runtime/cli) and Astral-level CLI UX.

**Architecture:** Single crate with clean module boundaries. Domain types import nothing. Ports define traits. Adapters implement them. Runtime wires everything. CLI consumes runtime.

**Tech Stack:** Rust, clap, miette, owo-colors, indicatif, dialoguer, tabled

---

## Phase 1: Module Restructure

Move existing code into new hexagonal structure without changing logic.

### Task 1: Create New Directory Structure

**Files:**
- Create: `src/domain/mod.rs`
- Create: `src/ports/mod.rs`
- Create: `src/adapters/mod.rs`
- Create: `src/runtime/mod.rs`

**Step 1: Create domain module**

```bash
mkdir -p src/domain
```

Create `src/domain/mod.rs`:

```rust
//! Pure domain types. No I/O, no external dependencies.

mod execution;
mod id;
mod market;
mod market_registry;
mod money;
mod monitoring;
mod opportunity;
mod order_book;
mod position;
mod relation;
mod resource;
mod scaling;
mod score;

pub use execution::{ArbitrageExecutionResult, FailedLeg, FilledLeg, OrderId};
pub use id::{ClusterId, MarketId, RelationId, TokenId};
pub use market::{Market, Outcome};
pub use market_registry::MarketRegistry;
pub use money::{Price, Volume};
pub use monitoring::PoolStats;
pub use opportunity::{Opportunity, OpportunityLeg};
pub use order_book::{OrderBook, PriceLevel};
pub use position::{Position, PositionId, PositionLeg, PositionStatus};
pub use relation::{Cluster, Relation, RelationKind};
pub use resource::ResourceBudget;
pub use scaling::ScalingRecommendation;
pub use score::{MarketScore, ScoreFactors, ScoreWeights};
```

**Step 2: Create ports module**

Create `src/ports/mod.rs`:

```rust
//! Trait definitions (hexagonal ports). Depend only on domain.

mod exchange;
mod inference;
mod notifier;
mod risk;
mod solver;
mod store;
mod strategy;

pub use exchange::{
    ArbitrageExecutor, ExecutionResult, MarketDataStream, MarketEvent, MarketFetcher,
    MarketInfo, OrderExecutor, OrderRequest, OrderSide, OutcomeInfo,
};
pub use inference::RelationInferrer;
pub use notifier::{Event, Notifier};
pub use risk::RiskGate;
pub use solver::Solver;
pub use store::Store;
pub use strategy::{DetectionContext, DetectionResult, MarketContext, Strategy};
```

**Step 3: Create adapters module**

Create `src/adapters/mod.rs`:

```rust
//! Implementations of ports (hexagonal adapters).

pub mod llm;
pub mod notifiers;
pub mod polymarket;
pub mod solvers;
pub mod stores;
pub mod strategies;
```

**Step 4: Create runtime module**

Create `src/runtime/mod.rs`:

```rust
//! Orchestration, configuration, and wiring.

mod builder;
mod config;
mod governor;
mod orchestrator;
mod state;

pub use builder::Builder;
pub use config::Config;
pub use governor::{AdaptiveGovernor, GovernorConfig, LatencyGovernor};
pub use orchestrator::Orchestrator;
pub use state::AppState;
```

**Step 5: Commit**

```bash
git add src/domain src/ports src/adapters src/runtime
git commit -m "chore: create hexagonal module structure"
```

---

### Task 2: Move Domain Types

**Files:**
- Move: `src/core/domain/*.rs` → `src/domain/`
- Modify: `src/domain/mod.rs`

**Step 1: Move all domain files**

```bash
mv src/core/domain/execution.rs src/domain/
mv src/core/domain/id.rs src/domain/
mv src/core/domain/market.rs src/domain/
mv src/core/domain/market_registry.rs src/domain/
mv src/core/domain/money.rs src/domain/
mv src/core/domain/monitoring.rs src/domain/
mv src/core/domain/opportunity.rs src/domain/
mv src/core/domain/order_book.rs src/domain/
mv src/core/domain/position.rs src/domain/
mv src/core/domain/relation.rs src/domain/
mv src/core/domain/resource.rs src/domain/
mv src/core/domain/scaling.rs src/domain/
mv src/core/domain/score.rs src/domain/
```

**Step 2: Update imports in moved files**

In each moved file, update any `crate::core::domain::` imports to `crate::domain::`.

**Step 3: Run tests**

```bash
cargo test
```

Expected: Tests pass (imports may need updating in other files).

**Step 4: Commit**

```bash
git add src/domain src/core/domain
git commit -m "refactor: move domain types to src/domain"
```

---

### Task 3: Extract Port Traits

**Files:**
- Create: `src/ports/strategy.rs`
- Create: `src/ports/exchange.rs`
- Create: `src/ports/notifier.rs`
- Create: `src/ports/store.rs`
- Create: `src/ports/solver.rs`
- Create: `src/ports/inference.rs`
- Create: `src/ports/risk.rs`

**Step 1: Extract Strategy trait**

Create `src/ports/strategy.rs`:

```rust
//! Strategy trait for arbitrage detection.

use std::sync::Arc;

use crate::domain::{MarketRegistry, Opportunity, OrderBook, Market};

/// Context for checking if a strategy applies to a market.
pub struct MarketContext<'a> {
    pub market: &'a Market,
    pub book: &'a OrderBook,
}

/// Full context for detection.
pub struct DetectionContext<'a> {
    pub market: &'a Market,
    pub book: &'a OrderBook,
    pub registry: &'a MarketRegistry,
}

/// Result from a detection pass.
pub struct DetectionResult {
    pub opportunities: Vec<Opportunity>,
}

/// A detection strategy that finds arbitrage opportunities.
pub trait Strategy: Send + Sync {
    /// Unique identifier for this strategy.
    fn name(&self) -> &'static str;

    /// Check if this strategy should run for a given market.
    fn applies_to(&self, ctx: &MarketContext) -> bool;

    /// Detect opportunities given current market state.
    fn detect(&self, ctx: &DetectionContext) -> Vec<Opportunity>;

    /// Optional: warm-start hint from previous detection.
    fn warm_start(&mut self, _previous: &DetectionResult) {}

    /// Optional: inject market registry for strategies that need it.
    fn set_market_registry(&mut self, _registry: Arc<MarketRegistry>) {}
}
```

**Step 2: Extract Exchange traits**

Create `src/ports/exchange.rs`:

```rust
//! Exchange abstraction traits.

use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::domain::{
    ArbitrageExecutionResult, Market, MarketId, Opportunity, OrderBook, TokenId,
};

/// Information about an outcome.
pub struct OutcomeInfo {
    pub token_id: TokenId,
    pub name: String,
    pub price: Decimal,
}

/// Information about a market.
pub struct MarketInfo {
    pub id: MarketId,
    pub question: String,
    pub outcomes: Vec<OutcomeInfo>,
    pub volume_24h: Decimal,
    pub liquidity: Decimal,
    pub end_date: Option<chrono::DateTime<chrono::Utc>>,
}

/// Market data event.
pub enum MarketEvent {
    BookUpdate { token_id: TokenId, book: OrderBook },
    Subscribed { token_id: TokenId },
    Unsubscribed { token_id: TokenId },
    Error { message: String },
}

/// Order side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Order request.
pub struct OrderRequest {
    pub token_id: TokenId,
    pub side: OrderSide,
    pub price: Decimal,
    pub size: Decimal,
}

/// Execution result for a single order.
pub struct ExecutionResult {
    pub filled_size: Decimal,
    pub filled_price: Decimal,
    pub fees: Decimal,
}

/// Fetches market information from an exchange.
#[async_trait]
pub trait MarketFetcher: Send + Sync {
    async fn fetch_markets(&self) -> crate::error::Result<Vec<Market>>;
}

/// Streams real-time market data.
#[async_trait]
pub trait MarketDataStream: Send + Sync {
    async fn subscribe(&mut self, token_ids: &[TokenId]) -> crate::error::Result<()>;
    async fn unsubscribe(&mut self, token_ids: &[TokenId]) -> crate::error::Result<()>;
    async fn next(&mut self) -> Option<MarketEvent>;
}

/// Executes orders on an exchange.
#[async_trait]
pub trait OrderExecutor: Send + Sync {
    async fn execute(&self, request: OrderRequest) -> crate::error::Result<ExecutionResult>;
}

/// Executes multi-leg arbitrage opportunities atomically.
#[async_trait]
pub trait ArbitrageExecutor: Send + Sync {
    async fn execute(&self, opportunity: &Opportunity) -> crate::error::Result<ArbitrageExecutionResult>;
}
```

**Step 3: Extract remaining port traits**

Create `src/ports/notifier.rs`:

```rust
//! Notification trait.

use async_trait::async_trait;

use crate::domain::Opportunity;

/// Notification event.
pub enum Event {
    OpportunityDetected(Opportunity),
    OpportunityExecuted { opportunity: Opportunity, profit: rust_decimal::Decimal },
    OpportunityRejected { opportunity: Opportunity, reason: String },
    Error(String),
}

/// Sends notifications.
#[async_trait]
pub trait Notifier: Send + Sync {
    async fn notify(&self, event: Event) -> crate::error::Result<()>;
}
```

Create `src/ports/store.rs`:

```rust
//! Persistence trait.

use async_trait::async_trait;

use crate::domain::{Relation, RelationId};

/// Persists and retrieves data.
#[async_trait]
pub trait Store: Send + Sync {
    async fn save_relation(&self, relation: &Relation) -> crate::error::Result<()>;
    async fn get_relation(&self, id: &RelationId) -> crate::error::Result<Option<Relation>>;
    async fn list_relations(&self) -> crate::error::Result<Vec<Relation>>;
}
```

Create `src/ports/solver.rs`:

```rust
//! LP/ILP solver trait.

use rust_decimal::Decimal;

/// Result from solving an optimization problem.
pub struct SolverResult {
    pub allocations: Vec<Decimal>,
    pub objective: Decimal,
}

/// Solves linear/integer programming problems.
pub trait Solver: Send + Sync {
    fn solve(&self, constraints: &[Constraint]) -> crate::error::Result<SolverResult>;
}

/// A linear constraint.
pub struct Constraint {
    pub coefficients: Vec<Decimal>,
    pub bound: Decimal,
}
```

Create `src/ports/inference.rs`:

```rust
//! Market relation inference trait.

use async_trait::async_trait;

use crate::domain::{Market, Relation};

/// Infers relationships between markets.
#[async_trait]
pub trait RelationInferrer: Send + Sync {
    async fn infer(&self, markets: &[Market]) -> crate::error::Result<Vec<Relation>>;
}
```

Create `src/ports/risk.rs`:

```rust
//! Risk management trait.

use crate::domain::Opportunity;

/// Result of a risk check.
pub enum RiskCheckResult {
    Approved,
    Rejected { reason: String },
}

/// Gates opportunities based on risk criteria.
pub trait RiskGate: Send + Sync {
    fn check(&self, opportunity: &Opportunity) -> RiskCheckResult;
}
```

**Step 4: Update ports/mod.rs with all exports**

**Step 5: Run tests**

```bash
cargo test
```

**Step 6: Commit**

```bash
git add src/ports
git commit -m "refactor: extract port traits"
```

---

### Task 4: Move Adapters

**Files:**
- Move: `src/core/exchange/polymarket/` → `src/adapters/polymarket/`
- Move: `src/core/strategy/` → `src/adapters/strategies/`
- Move: `src/core/service/messaging/` → `src/adapters/notifiers/`
- Move: `src/core/store/` → `src/adapters/stores/`
- Move: `src/core/solver/` → `src/adapters/solvers/`
- Move: `src/core/llm/` → `src/adapters/llm/`

**Step 1: Move polymarket adapter**

```bash
mv src/core/exchange/polymarket src/adapters/polymarket
```

**Step 2: Move strategy implementations**

```bash
mkdir -p src/adapters/strategies
mv src/core/strategy/condition src/adapters/strategies/
mv src/core/strategy/rebalancing src/adapters/strategies/
mv src/core/strategy/combinatorial src/adapters/strategies/
mv src/core/strategy/registry.rs src/adapters/strategies/
mv src/core/strategy/context.rs src/adapters/strategies/
```

**Step 3: Move notifier implementations**

```bash
mv src/core/service/messaging src/adapters/notifiers
```

**Step 4: Move store implementations**

```bash
mv src/core/store src/adapters/stores
```

**Step 5: Move solver implementations**

```bash
mv src/core/solver src/adapters/solvers
```

**Step 6: Move LLM implementations**

```bash
mv src/core/llm src/adapters/llm
```

**Step 7: Update imports in all moved files**

Replace `crate::core::` with appropriate new paths.

**Step 8: Run tests**

```bash
cargo test
```

**Step 9: Commit**

```bash
git add src/adapters src/core
git commit -m "refactor: move adapter implementations"
```

---

### Task 5: Move Runtime Components

**Files:**
- Move: `src/app/orchestrator/` → `src/runtime/`
- Move: `src/app/config/` → `src/runtime/`
- Move: `src/app/state.rs` → `src/runtime/`
- Move: `src/core/service/governor/` → `src/runtime/`

**Step 1: Move orchestrator**

```bash
mv src/app/orchestrator/* src/runtime/
```

**Step 2: Move config**

```bash
mv src/app/config/* src/runtime/
```

**Step 3: Move state**

```bash
mv src/app/state.rs src/runtime/
```

**Step 4: Move governor**

```bash
mv src/core/service/governor/* src/runtime/
```

**Step 5: Create runtime builder**

Create `src/runtime/builder.rs`:

```rust
//! Application builder for composing components.

use std::sync::Arc;

use crate::ports::{Strategy, Notifier, RiskGate, ArbitrageExecutor};
use crate::runtime::{Config, Orchestrator};

/// Builder for constructing the application.
pub struct Builder {
    config: Option<Config>,
    strategies: Vec<Box<dyn Strategy>>,
    notifiers: Vec<Arc<dyn Notifier>>,
    risk_gate: Option<Arc<dyn RiskGate>>,
    executor: Option<Arc<dyn ArbitrageExecutor>>,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            config: None,
            strategies: Vec::new(),
            notifiers: Vec::new(),
            risk_gate: None,
            executor: None,
        }
    }

    pub fn config(mut self, config: Config) -> Self {
        self.config = Some(config);
        self
    }

    pub fn strategy(mut self, strategy: impl Strategy + 'static) -> Self {
        self.strategies.push(Box::new(strategy));
        self
    }

    pub fn notifier(mut self, notifier: Arc<dyn Notifier>) -> Self {
        self.notifiers.push(notifier);
        self
    }

    pub fn risk_gate(mut self, gate: Arc<dyn RiskGate>) -> Self {
        self.risk_gate = Some(gate);
        self
    }

    pub fn executor(mut self, executor: Arc<dyn ArbitrageExecutor>) -> Self {
        self.executor = Some(executor);
        self
    }

    pub fn build(self) -> crate::error::Result<Orchestrator> {
        let config = self.config.ok_or_else(|| {
            crate::error::Error::Config("config required".into())
        })?;

        Orchestrator::new(config, self.strategies, self.notifiers, self.risk_gate, self.executor)
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 6: Update imports**

**Step 7: Run tests**

```bash
cargo test
```

**Step 8: Commit**

```bash
git add src/runtime src/app src/core
git commit -m "refactor: move runtime components"
```

---

### Task 6: Move Remaining Core Services

**Files:**
- Move: `src/core/service/risk.rs` → `src/adapters/risk.rs`
- Move: `src/core/service/statistics/` → `src/adapters/statistics/`
- Move: `src/core/service/position/` → `src/adapters/position/`
- Move: `src/core/service/cluster/` → `src/adapters/cluster/`
- Move: `src/core/service/inference/` → `src/adapters/inference/`
- Move: `src/core/service/subscription/` → `src/runtime/subscription/`
- Move: `src/core/cache/` → `src/runtime/cache/`
- Move: `src/core/db/` → `src/adapters/stores/db/`
- Move: `src/core/inference/` → `src/adapters/inference/`

**Step 1: Move service implementations to adapters**

```bash
mv src/core/service/risk.rs src/adapters/
mv src/core/service/statistics src/adapters/
mv src/core/service/position src/adapters/
mv src/core/service/cluster src/adapters/
mv src/core/service/inference src/adapters/
```

**Step 2: Move runtime-specific services**

```bash
mv src/core/service/subscription src/runtime/
mv src/core/cache src/runtime/
```

**Step 3: Move database to stores adapter**

```bash
mv src/core/db src/adapters/stores/
```

**Step 4: Move exchange abstractions to runtime**

```bash
mkdir -p src/runtime/exchange
mv src/core/exchange/pool.rs src/runtime/exchange/
mv src/core/exchange/reconnecting.rs src/runtime/exchange/
mv src/core/exchange/factory.rs src/runtime/exchange/
mv src/core/exchange/filter.rs src/runtime/exchange/
mv src/core/exchange/scorer.rs src/runtime/exchange/
```

**Step 5: Update all imports**

**Step 6: Run tests**

```bash
cargo test
```

**Step 7: Commit**

```bash
git add .
git commit -m "refactor: move remaining services"
```

---

### Task 7: Clean Up Old Structure

**Files:**
- Delete: `src/core/` (should be empty or have only mod.rs)
- Delete: `src/app/` (should be empty or have only mod.rs)
- Modify: `src/lib.rs`

**Step 1: Remove old core module**

```bash
rm -rf src/core
```

**Step 2: Remove old app module**

```bash
rm -rf src/app
```

**Step 3: Update lib.rs**

```rust
//! Edgelord - Prediction market arbitrage framework.
//!
//! # For CLI users
//!
//! Install and run:
//!
//!     cargo install edgelord
//!     edgelord init
//!     edgelord run
//!
//! # For developers
//!
//! Fork this repo and extend:
//!
//! - Add strategies: implement `ports::Strategy`
//! - Add exchanges: implement `ports::MarketDataStream` + `ports::ArbitrageExecutor`
//! - Add notifiers: implement `ports::Notifier`
//!
//! # Architecture
//!
//! ```text
//! domain/     Pure types, no I/O
//! ports/      Trait definitions (extension points)
//! adapters/   Implementations (Polymarket, strategies, etc.)
//! runtime/    Orchestration and wiring
//! cli/        Command-line interface
//! ```

pub mod domain;
pub mod ports;
pub mod adapters;
pub mod runtime;
pub mod cli;
pub mod error;

#[cfg(any(test, feature = "testkit"))]
pub mod testkit;
```

**Step 4: Run tests**

```bash
cargo test
```

**Step 5: Commit**

```bash
git add .
git commit -m "refactor: remove old module structure"
```

---

### Task 8: Update All Import Paths

**Files:**
- Modify: All `.rs` files with `crate::core::` or `crate::app::` imports

**Step 1: Find and replace imports**

Run this search to find files needing updates:

```bash
grep -r "crate::core::" src/
grep -r "crate::app::" src/
```

**Step 2: Update each file**

Common replacements:
- `crate::core::domain::` → `crate::domain::`
- `crate::core::strategy::` → `crate::ports::` (for traits) or `crate::adapters::strategies::` (for impls)
- `crate::core::exchange::` → `crate::ports::` (for traits) or `crate::adapters::polymarket::` (for impls)
- `crate::app::Config` → `crate::runtime::Config`
- `crate::app::Orchestrator` → `crate::runtime::Orchestrator`

**Step 3: Run tests**

```bash
cargo test
```

**Step 4: Commit**

```bash
git add .
git commit -m "refactor: update all import paths"
```

---

## Phase 2: CLI Overhaul

Add Astral-level CLI output formatting.

### Task 9: Add CLI Dependencies

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add new dependencies**

Add to `[dependencies]`:

```toml
miette = { version = "7", features = ["fancy"] }
owo-colors = "4"
indicatif = "0.17"
dialoguer = "0.11"
tabled = "0.16"
```

**Step 2: Run build**

```bash
cargo build
```

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "deps: add CLI UX libraries"
```

---

### Task 10: Create Output Module

**Files:**
- Rewrite: `src/cli/output.rs`

**Step 1: Rewrite output module**

```rust
//! Astral-style CLI output formatting.

use std::fmt::Display;
use std::io::{self, Write};

use owo_colors::OwoColorize;

/// Print the application header.
pub fn header(version: &str) {
    println!("{} {}", "edgelord".bold(), version.dimmed());
    println!();
}

/// Print a labeled value.
pub fn field(label: &str, value: impl Display) {
    println!("  {:<12} {}", label.dimmed(), value);
}

/// Print a success line.
pub fn success(message: &str) {
    println!("  {} {}", "✓".green(), message);
}

/// Print a warning line.
pub fn warning(message: &str) {
    println!("  {} {}", "⚠".yellow(), message);
}

/// Print an error line.
pub fn error(message: &str) {
    eprintln!("  {} {}", "×".red(), message);
}

/// Print a section header.
pub fn section(title: &str) {
    println!();
    println!("{}", title.bold());
}

/// Print an info line (for streaming output).
pub fn info(timestamp: &str, label: &str, message: &str) {
    println!(
        "  {} {} {}",
        timestamp.dimmed(),
        label.cyan(),
        message
    );
}

/// Print an executed trade line.
pub fn executed(timestamp: &str, message: &str) {
    println!(
        "  {} {} {}",
        timestamp.dimmed(),
        "executed".green(),
        message
    );
}

/// Print a rejected opportunity line.
pub fn rejected(timestamp: &str, reason: &str) {
    println!(
        "  {} {} {}",
        timestamp.dimmed(),
        "rejected".red(),
        reason
    );
}

/// Print an opportunity line.
pub fn opportunity(timestamp: &str, message: &str) {
    println!(
        "  {} {} {}",
        timestamp.dimmed(),
        "opportunity".yellow(),
        message
    );
}

/// Start a progress spinner.
pub fn spinner(message: &str) -> indicatif::ProgressBar {
    let pb = indicatif::ProgressBar::new_spinner();
    pb.set_style(
        indicatif::ProgressStyle::default_spinner()
            .template("  {spinner:.cyan} {msg}")
            .unwrap()
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb
}

/// Finish a spinner with success.
pub fn spinner_success(pb: &indicatif::ProgressBar, message: &str) {
    pb.finish_with_message(format!("{} {}", "✓".green(), message));
}

/// Finish a spinner with failure.
pub fn spinner_fail(pb: &indicatif::ProgressBar, message: &str) {
    pb.finish_with_message(format!("{} {}", "×".red(), message));
}

/// Format a positive value in green.
pub fn positive(value: impl Display) -> String {
    format!("{}", value.to_string().green())
}

/// Format a negative value in red.
pub fn negative(value: impl Display) -> String {
    format!("{}", value.to_string().red())
}

/// Format a highlighted value in cyan.
pub fn highlight(value: impl Display) -> String {
    format!("{}", value.to_string().cyan())
}
```

**Step 2: Run build**

```bash
cargo build
```

**Step 3: Commit**

```bash
git add src/cli/output.rs
git commit -m "feat(cli): add Astral-style output formatting"
```

---

### Task 11: Create Error Diagnostics

**Files:**
- Create: `src/cli/diagnostic.rs`
- Modify: `src/error.rs`

**Step 1: Create diagnostic module**

Create `src/cli/diagnostic.rs`:

```rust
//! Miette-based error diagnostics.

use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

/// Configuration error with source location.
#[derive(Debug, Error, Diagnostic)]
#[error("{message}")]
#[diagnostic(code(edgelord::config))]
pub struct ConfigError {
    pub message: String,

    #[source_code]
    pub src: String,

    #[label("here")]
    pub span: SourceSpan,

    #[help]
    pub help: Option<String>,
}

impl ConfigError {
    pub fn new(message: impl Into<String>, src: impl Into<String>, offset: usize, len: usize) -> Self {
        Self {
            message: message.into(),
            src: src.into(),
            span: (offset, len).into(),
            help: None,
        }
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

/// Strategy error.
#[derive(Debug, Error, Diagnostic)]
#[error("{message}")]
#[diagnostic(code(edgelord::strategy))]
pub struct StrategyError {
    pub message: String,

    #[help]
    pub help: Option<String>,
}

/// Connection error.
#[derive(Debug, Error, Diagnostic)]
#[error("connection failed: {message}")]
#[diagnostic(
    code(edgelord::connection),
    help("check your network connection and exchange status")
)]
pub struct ConnectionError {
    pub message: String,
}
```

**Step 2: Update error.rs to use miette**

Add to `src/error.rs`:

```rust
use miette::Diagnostic;

// Add #[derive(Diagnostic)] to Error enum
```

**Step 3: Commit**

```bash
git add src/cli/diagnostic.rs src/error.rs
git commit -m "feat(cli): add miette error diagnostics"
```

---

### Task 12: Add Init Command

**Files:**
- Create: `src/cli/init.rs`
- Modify: `src/cli/mod.rs`

**Step 1: Create init command**

Create `src/cli/init.rs`:

```rust
//! Interactive setup wizard.

use std::path::PathBuf;

use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};

use crate::cli::output;
use crate::error::Result;

/// Run the interactive setup wizard.
pub fn execute(path: PathBuf, force: bool) -> Result<()> {
    output::header(env!("CARGO_PKG_VERSION"));
    println!("  Welcome to edgelord\n");
    println!("  Let's set up your configuration.\n");

    let theme = ColorfulTheme::default();

    // Network selection
    let networks = &["Testnet (recommended for first run)", "Mainnet"];
    let network = Select::with_theme(&theme)
        .with_prompt("  Network")
        .items(networks)
        .default(0)
        .interact()?;

    let environment = if network == 0 { "testnet" } else { "mainnet" };

    // Strategy selection
    let strategies = &[
        ("single-condition", "YES + NO < $1 arbitrage", true),
        ("market-rebalancing", "Multi-outcome rebalancing", true),
        ("combinatorial", "Cross-market (requires LLM)", false),
    ];

    println!("\n  Strategies");
    let mut enabled_strategies = Vec::new();
    for (name, desc, default) in strategies {
        let enabled = Confirm::with_theme(&theme)
            .with_prompt(format!("    {} - {}", name, desc))
            .default(*default)
            .interact()?;
        if enabled {
            enabled_strategies.push(*name);
        }
    }

    // Risk settings
    println!("\n  Risk limits");
    let max_exposure: f64 = Input::with_theme(&theme)
        .with_prompt("    Maximum total exposure ($)")
        .default(500.0)
        .interact()?;

    let max_position: f64 = Input::with_theme(&theme)
        .with_prompt("    Maximum per-market position ($)")
        .default(100.0)
        .interact()?;

    // Generate config
    let config = generate_config(environment, &enabled_strategies, max_exposure, max_position);

    // Check if file exists
    if path.exists() && !force {
        let overwrite = Confirm::with_theme(&theme)
            .with_prompt(format!("  {} already exists. Overwrite?", path.display()))
            .default(false)
            .interact()?;
        if !overwrite {
            println!("\n  Aborted.");
            return Ok(());
        }
    }

    // Write config
    std::fs::write(&path, config)?;

    println!();
    output::success(&format!("Created {}", path.display()));
    println!();
    println!("  Next steps:");
    println!("    1. Set EDGELORD_WALLET_KEY environment variable");
    println!("    2. Run {} to validate", output::highlight("edgelord check live"));
    println!("    3. Run {} to start", output::highlight("edgelord run"));
    println!();

    Ok(())
}

fn generate_config(
    environment: &str,
    strategies: &[&str],
    max_exposure: f64,
    max_position: f64,
) -> String {
    let strategies_list = strategies.iter()
        .map(|s| format!("\"{}\"", s))
        .collect::<Vec<_>>()
        .join(", ");

    format!(r#"# Edgelord configuration
# Generated by `edgelord init`

[network]
environment = "{environment}"

[strategies]
enabled = [{strategies_list}]

[strategies.single-condition]
min_edge = 0.05
min_profit = 0.50

[strategies.market-rebalancing]
min_edge = 0.03

[risk]
max_exposure = {max_exposure}
max_position_per_market = {max_position}
max_slippage = 0.02

[notifications]
backend = "none"

[database]
path = "edgelord.db"

[governor]
enabled = true
target_latency_p99_ms = 100
"#)
}
```

**Step 2: Add init to CLI**

Add to `src/cli/mod.rs` Commands enum:

```rust
/// Initialize configuration interactively
Init(InitArgs),
```

Add args struct:

```rust
#[derive(Parser, Debug)]
pub struct InitArgs {
    /// Output path for config file
    #[arg(default_value = "config.toml")]
    pub path: PathBuf,

    /// Overwrite if file exists
    #[arg(long)]
    pub force: bool,
}
```

**Step 3: Wire up in main.rs**

**Step 4: Run test**

```bash
cargo run -- init --help
```

**Step 5: Commit**

```bash
git add src/cli/init.rs src/cli/mod.rs src/main.rs
git commit -m "feat(cli): add interactive init command"
```

---

### Task 13: Add Strategies Command

**Files:**
- Create: `src/cli/strategies.rs`
- Modify: `src/cli/mod.rs`

**Step 1: Create strategies command**

Create `src/cli/strategies.rs`:

```rust
//! Strategy listing and explanation.

use tabled::{Table, Tabled};

use crate::cli::output;
use crate::error::Result;

#[derive(Tabled)]
struct StrategyRow {
    #[tabled(rename = "Name")]
    name: &'static str,
    #[tabled(rename = "Signal")]
    signal: &'static str,
    #[tabled(rename = "Typical Edge")]
    edge: &'static str,
}

/// List available strategies.
pub fn list() -> Result<()> {
    output::header(env!("CARGO_PKG_VERSION"));
    output::section("Available strategies");
    println!();

    let strategies = vec![
        StrategyRow {
            name: "single-condition",
            signal: "YES + NO < $1",
            edge: "2-5%",
        },
        StrategyRow {
            name: "market-rebalancing",
            signal: "sum(outcomes) < $1",
            edge: "1-3%",
        },
        StrategyRow {
            name: "combinatorial",
            signal: "cross-market constraints",
            edge: "<1%",
        },
    ];

    let table = Table::new(strategies).to_string();
    for line in table.lines() {
        println!("  {}", line);
    }

    println!();
    println!("  Run {} for details", output::highlight("edgelord strategies explain <name>"));
    println!();

    Ok(())
}

/// Explain a specific strategy.
pub fn explain(name: &str) -> Result<()> {
    output::header(env!("CARGO_PKG_VERSION"));

    match name {
        "single-condition" => explain_single_condition(),
        "market-rebalancing" => explain_market_rebalancing(),
        "combinatorial" => explain_combinatorial(),
        _ => {
            output::error(&format!("Unknown strategy: {}", name));
            println!();
            println!("  Available: single-condition, market-rebalancing, combinatorial");
            return Ok(());
        }
    }

    Ok(())
}

fn explain_single_condition() {
    output::section("single-condition");
    println!();
    println!("  Detects arbitrage in binary (YES/NO) markets where:");
    println!("  YES price + NO price < $1.00 payout");
    println!();
    println!("  Example:");
    println!("    YES @ $0.45 + NO @ $0.52 = $0.97");
    println!("    Payout = $1.00");
    println!("    Edge = $0.03 (3%)");
    println!();
    println!("  Configuration:");
    println!("    [strategies.single-condition]");
    println!("    min_edge = 0.05    # 5% minimum edge");
    println!("    min_profit = 0.50  # $0.50 minimum profit");
    println!();
}

fn explain_market_rebalancing() {
    output::section("market-rebalancing");
    println!();
    println!("  Detects arbitrage in multi-outcome markets where:");
    println!("  sum(all outcome prices) < $1.00 payout");
    println!();
    println!("  Example (3-outcome market):");
    println!("    Option A @ $0.30 + Option B @ $0.35 + Option C @ $0.32 = $0.97");
    println!("    Payout = $1.00");
    println!("    Edge = $0.03 (3%)");
    println!();
    println!("  Configuration:");
    println!("    [strategies.market-rebalancing]");
    println!("    min_edge = 0.03");
    println!();
}

fn explain_combinatorial() {
    output::section("combinatorial");
    println!();
    println!("  Detects arbitrage across related markets using:");
    println!("  - LLM inference to identify market relationships");
    println!("  - LP/ILP optimization to find profitable combinations");
    println!();
    println!("  Example:");
    println!("    Market A: \"Will X happen in 2024?\"");
    println!("    Market B: \"Will X happen in Q4 2024?\"");
    println!("    Constraint: B implies A");
    println!();
    println!("  Requires:");
    println!("    [inference]");
    println!("    provider = \"anthropic\"  # or \"openai\"");
    println!();
    println!("  Configuration:");
    println!("    [strategies.combinatorial]");
    println!("    min_edge = 0.02");
    println!();
}
```

**Step 2: Add to CLI**

**Step 3: Wire up in main.rs**

**Step 4: Test**

```bash
cargo run -- strategies list
cargo run -- strategies explain single-condition
```

**Step 5: Commit**

```bash
git add src/cli/strategies.rs src/cli/mod.rs src/main.rs
git commit -m "feat(cli): add strategies list/explain commands"
```

---

### Task 14: Update Run Command Output

**Files:**
- Modify: `src/cli/run.rs`

**Step 1: Update run command to use new output**

Replace startup output with Astral-style formatting using output module.

**Step 2: Add spinners for market fetching**

**Step 3: Format opportunity/execution logs consistently**

**Step 4: Test**

```bash
cargo run -- run --dry-run --config config.toml
```

**Step 5: Commit**

```bash
git add src/cli/run.rs
git commit -m "feat(cli): update run command with Astral-style output"
```

---

### Task 15: Update Status Command Output

**Files:**
- Modify: `src/cli/status.rs`

**Step 1: Update status output formatting**

Use tabled for statistics, owo-colors for values.

**Step 2: Test**

```bash
cargo run -- status
```

**Step 3: Commit**

```bash
git add src/cli/status.rs
git commit -m "feat(cli): update status command with Astral-style output"
```

---

### Task 16: Add Global Flags

**Files:**
- Modify: `src/cli/mod.rs`
- Modify: `src/main.rs`

**Step 1: Add global flags to Cli struct**

```rust
#[derive(Parser, Debug)]
#[command(name = "edgelord")]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Output format
    #[arg(long, global = true, default_value = "auto")]
    pub color: ColorChoice,

    /// JSON output for scripting
    #[arg(long, global = true)]
    pub json: bool,

    /// Decrease output verbosity
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Increase output verbosity
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Clone, Debug, Default, clap::ValueEnum)]
pub enum ColorChoice {
    #[default]
    Auto,
    Always,
    Never,
}
```

**Step 2: Apply color settings in main.rs**

```rust
fn setup_colors(choice: ColorChoice) {
    match choice {
        ColorChoice::Auto => {
            // owo-colors auto-detects TTY
        }
        ColorChoice::Always => {
            owo_colors::set_override(true);
        }
        ColorChoice::Never => {
            owo_colors::set_override(false);
        }
    }
}
```

**Step 3: Test**

```bash
cargo run -- --color never status
cargo run -- --json status
```

**Step 4: Commit**

```bash
git add src/cli/mod.rs src/main.rs
git commit -m "feat(cli): add global --color, --json, --quiet, --verbose flags"
```

---

## Phase 3: Documentation

Update all documentation to match new style.

### Task 17: Rewrite README

**Files:**
- Rewrite: `README.md`

**Step 1: Write new README**

Concise, no slop, Astral-style. See design document for template.

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: rewrite README"
```

---

### Task 18: Update Extending Docs

**Files:**
- Create: `docs/extending/strategies.md`
- Create: `docs/extending/exchanges.md`
- Create: `docs/extending/architecture.md`

**Step 1: Write strategy extension guide**

**Step 2: Write exchange extension guide**

**Step 3: Write architecture overview**

**Step 4: Commit**

```bash
git add docs/extending/
git commit -m "docs: add extension guides"
```

---

### Task 19: Add Doc Comments

**Files:**
- All public items in `src/domain/`, `src/ports/`

**Step 1: Audit public API**

```bash
cargo doc --no-deps 2>&1 | grep "warning: missing documentation"
```

**Step 2: Add missing doc comments**

**Step 3: Build docs**

```bash
cargo doc --no-deps
```

**Step 4: Commit**

```bash
git add src/
git commit -m "docs: add doc comments to public API"
```

---

## Phase 4: Polish

Final cleanup and testing.

### Task 20: Integration Tests for CLI Output

**Files:**
- Create: `tests/cli_output_tests.rs`

**Step 1: Write tests for CLI output**

Test that commands produce expected output format.

**Step 2: Run tests**

```bash
cargo test cli_output
```

**Step 3: Commit**

```bash
git add tests/cli_output_tests.rs
git commit -m "test: add CLI output integration tests"
```

---

### Task 21: Test cargo install Flow

**Files:**
- None (manual testing)

**Step 1: Build release**

```bash
cargo build --release
```

**Step 2: Test local install**

```bash
cargo install --path .
```

**Step 3: Verify commands work**

```bash
edgelord --help
edgelord init --help
edgelord strategies list
edgelord check config config.toml
```

**Step 4: Note any issues for fixing**

---

### Task 22: Final Cleanup

**Files:**
- Various

**Step 1: Remove dead code**

```bash
cargo clippy -- -D dead_code
```

**Step 2: Format code**

```bash
cargo fmt
```

**Step 3: Run full test suite**

```bash
cargo test
```

**Step 4: Final commit**

```bash
git add .
git commit -m "chore: final cleanup"
```

---

## Summary

| Phase | Tasks | Focus |
|-------|-------|-------|
| 1 | 1-8 | Module restructure (hexagonal) |
| 2 | 9-16 | CLI UX overhaul (Astral-style) |
| 3 | 17-19 | Documentation |
| 4 | 20-22 | Polish and testing |

Total: 22 tasks across 4 phases.
