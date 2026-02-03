# Exchange Abstraction Cleanup Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Complete the exchange abstraction by fixing hardcoded outcome names, decoupling orchestrator from PolymarketExecutor, reorganizing config, and consistently prefixing all Polymarket types.

**Architecture:** Four main changes: (1) Strategy uses outcome indices instead of hardcoded "Yes"/"No", (2) Orchestrator uses `dyn ArbitrageExecutor` trait instead of concrete type, (3) App config uses enum for exchange-specific settings, (4) All Polymarket module types get consistent `Polymarket` prefix.

**Tech Stack:** Rust, async_trait, serde

---

### Task 1: Rename Polymarket Types with Consistent Prefix

**Files:**
- Modify: `src/core/exchange/polymarket/client.rs` - `Client` → `PolymarketClient`
- Modify: `src/core/exchange/polymarket/websocket.rs` - `DataStream` → `PolymarketDataStream`, `WebSocketHandler` → `PolymarketWebSocketHandler`
- Modify: `src/core/exchange/polymarket/executor.rs` - `Executor` → `PolymarketExecutor`
- Modify: `src/core/exchange/polymarket/mod.rs` - Update re-exports
- Modify: `src/core/exchange/factory.rs` - Update usages
- Modify: `src/app/orchestrator.rs` - Update import alias

**Step 1: Rename structs in client.rs**

Change:
```rust
pub struct Client {
```
To:
```rust
pub struct PolymarketClient {
```

Update all `Client` references to `PolymarketClient` (impl blocks, etc.)

**Step 2: Rename structs in websocket.rs**

Change:
```rust
pub struct WebSocketHandler {
```
To:
```rust
pub struct PolymarketWebSocketHandler {
```

Change:
```rust
pub struct DataStream {
```
To:
```rust
pub struct PolymarketDataStream {
```

Update all references in impl blocks.

**Step 3: Rename struct in executor.rs**

Change:
```rust
pub struct Executor {
```
To:
```rust
pub struct PolymarketExecutor {
```

Update all references in impl blocks.

**Step 4: Update mod.rs re-exports**

Change:
```rust
pub use client::Client;
pub use executor::Executor;
pub use websocket::{DataStream, WebSocketHandler};
```
To:
```rust
pub use client::PolymarketClient;
pub use executor::PolymarketExecutor;
pub use websocket::{PolymarketDataStream, PolymarketWebSocketHandler};
```

**Step 5: Update factory.rs**

Change references from `super::polymarket::Client` to `super::polymarket::PolymarketClient`, etc.

**Step 6: Update orchestrator.rs import**

Change:
```rust
use crate::core::exchange::polymarket::Executor as PolymarketExecutor;
```
To:
```rust
use crate::core::exchange::polymarket::PolymarketExecutor;
```

**Step 7: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 8: Commit**

```bash
git add -A
git commit -m "refactor(exchange): prefix Polymarket types consistently"
```

---

### Task 2: Fix SingleCondition Strategy Hardcoded Outcome Names

**Files:**
- Modify: `src/core/strategy/single_condition.rs:92-144`

**Problem:** Strategy uses `market.outcome_by_name("Yes")` and `market.outcome_by_name("No")` which fails on exchanges with different outcome names (e.g., "True"/"False").

**Solution:** Use outcome indices (0 and 1) since binary markets always have exactly 2 outcomes. The Market.outcomes() returns them in consistent order (positive first, negative second) as established by ExchangeConfig.parse_markets().

**Step 1: Update detect_single_condition function**

Change lines 97-99 from:
```rust
    // Get YES and NO outcomes
    let yes_outcome = market.outcome_by_name("Yes")?;
    let no_outcome = market.outcome_by_name("No")?;
```
To:
```rust
    // Get outcomes by index (binary markets have exactly 2 outcomes)
    // Index 0 = positive outcome, Index 1 = negative outcome
    let outcomes = market.outcomes();
    if outcomes.len() != 2 {
        return None;
    }
    let positive_outcome = &outcomes[0];
    let negative_outcome = &outcomes[1];
```

**Step 2: Update token references**

Change lines 101-102 from:
```rust
    let yes_book = cache.get(yes_outcome.token_id())?;
    let no_book = cache.get(no_outcome.token_id())?;
```
To:
```rust
    let positive_book = cache.get(positive_outcome.token_id())?;
    let negative_book = cache.get(negative_outcome.token_id())?;
```

**Step 3: Update ask price references**

Change lines 104-105 from:
```rust
    let yes_ask = yes_book.best_ask()?;
    let no_ask = no_book.best_ask()?;
```
To:
```rust
    let positive_ask = positive_book.best_ask()?;
    let negative_ask = negative_book.best_ask()?;
```

**Step 4: Update total_cost calculation**

Change line 107 from:
```rust
    let total_cost = yes_ask.price() + no_ask.price();
```
To:
```rust
    let total_cost = positive_ask.price() + negative_ask.price();
```

**Step 5: Update volume calculation**

Change line 123 from:
```rust
    let volume = yes_ask.size().min(no_ask.size());
```
To:
```rust
    let volume = positive_ask.size().min(negative_ask.size());
```

**Step 6: Update legs construction**

Change lines 132-135 from:
```rust
    let legs = vec![
        OpportunityLeg::new(yes_outcome.token_id().clone(), yes_ask.price()),
        OpportunityLeg::new(no_outcome.token_id().clone(), no_ask.price()),
    ];
```
To:
```rust
    let legs = vec![
        OpportunityLeg::new(positive_outcome.token_id().clone(), positive_ask.price()),
        OpportunityLeg::new(negative_outcome.token_id().clone(), negative_ask.price()),
    ];
```

**Step 7: Update docstring**

Change line 86 from:
```rust
/// * `market` - The binary YES/NO market
```
To:
```rust
/// * `market` - A binary market with exactly 2 outcomes
```

**Step 8: Update tests to not rely on outcome names for token lookup**

The tests use `market.outcome_by_name("Yes")` to get token IDs. Update to use indices:

Change test helper pattern from:
```rust
let yes_token = market.outcome_by_name("Yes").unwrap().token_id();
let no_token = market.outcome_by_name("No").unwrap().token_id();
```
To:
```rust
let outcomes = market.outcomes();
let positive_token = outcomes[0].token_id();
let negative_token = outcomes[1].token_id();
```

**Step 9: Run tests**

Run: `cargo test single_condition`
Expected: All tests pass

**Step 10: Commit**

```bash
git add src/core/strategy/single_condition.rs
git commit -m "fix(strategy): use outcome indices instead of hardcoded names"
```

---

### Task 3: Decouple Orchestrator from PolymarketExecutor

**Files:**
- Modify: `src/core/exchange/mod.rs` - Ensure ArbitrageExecutor is exported
- Modify: `src/core/exchange/factory.rs` - Add `create_arbitrage_executor()` method
- Modify: `src/app/orchestrator.rs` - Use `Arc<dyn ArbitrageExecutor>` instead of `Arc<PolymarketExecutor>`

**Step 1: Add create_arbitrage_executor to factory.rs**

Add after `create_executor`:
```rust
    /// Create an arbitrage executor for the configured exchange.
    ///
    /// Returns `None` if no wallet is configured.
    pub async fn create_arbitrage_executor(config: &Config) -> Result<Option<Arc<dyn ArbitrageExecutor + Send + Sync>>> {
        if config.wallet.private_key.is_none() {
            return Ok(None);
        }

        match config.exchange {
            Exchange::Polymarket => {
                let executor = super::polymarket::PolymarketExecutor::new(config).await?;
                Ok(Some(Arc::new(executor)))
            }
        }
    }
```

Add import at top:
```rust
use std::sync::Arc;
use super::ArbitrageExecutor;
```

**Step 2: Update orchestrator.rs imports**

Remove:
```rust
use crate::core::exchange::polymarket::Executor as PolymarketExecutor;
```

Keep:
```rust
use crate::core::exchange::{ArbitrageExecutionResult, ArbitrageExecutor};
```

**Step 3: Update init_executor function signature and body**

Change:
```rust
async fn init_executor(config: &Config) -> Option<Arc<PolymarketExecutor>> {
    if config.wallet.private_key.is_some() {
        match PolymarketExecutor::new(config).await {
            Ok(exec) => {
                info!("Executor initialized - trading ENABLED");
                Some(Arc::new(exec))
            }
            Err(e) => {
                warn!(error = %e, "Failed to initialize executor - detection only");
                None
            }
        }
    } else {
        info!("No wallet configured - detection only mode");
        None
    }
}
```
To:
```rust
async fn init_executor(config: &Config) -> Option<Arc<dyn ArbitrageExecutor + Send + Sync>> {
    match ExchangeFactory::create_arbitrage_executor(config).await {
        Ok(Some(exec)) => {
            info!("Executor initialized - trading ENABLED");
            Some(exec)
        }
        Ok(None) => {
            info!("No wallet configured - detection only mode");
            None
        }
        Err(e) => {
            warn!(error = %e, "Failed to initialize executor - detection only");
            None
        }
    }
}
```

**Step 4: Update handle_market_event signature**

Change:
```rust
fn handle_market_event(
    event: MarketEvent,
    cache: &OrderBookCache,
    registry: &MarketRegistry,
    strategies: &StrategyRegistry,
    executor: Option<Arc<PolymarketExecutor>>,
    ...
)
```
To:
```rust
fn handle_market_event(
    event: MarketEvent,
    cache: &OrderBookCache,
    registry: &MarketRegistry,
    strategies: &StrategyRegistry,
    executor: Option<Arc<dyn ArbitrageExecutor + Send + Sync>>,
    ...
)
```

**Step 5: Update handle_opportunity signature**

Change:
```rust
fn handle_opportunity(
    opp: Opportunity,
    executor: Option<Arc<PolymarketExecutor>>,
    ...
)
```
To:
```rust
fn handle_opportunity(
    opp: Opportunity,
    executor: Option<Arc<dyn ArbitrageExecutor + Send + Sync>>,
    ...
)
```

**Step 6: Update spawn_execution signature**

Change:
```rust
fn spawn_execution(
    executor: Arc<PolymarketExecutor>,
    ...
)
```
To:
```rust
fn spawn_execution(
    executor: Arc<dyn ArbitrageExecutor + Send + Sync>,
    ...
)
```

**Step 7: Remove comment about Polymarket-specific**

Remove line 74:
```rust
        // Initialize executor (optional) - still Polymarket-specific for now
```

**Step 8: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 9: Commit**

```bash
git add src/core/exchange/factory.rs src/app/orchestrator.rs
git commit -m "refactor(orchestrator): use ArbitrageExecutor trait instead of concrete type"
```

---

### Task 4: Reorganize App Config with Exchange-Specific Enum

**Files:**
- Modify: `src/app/config.rs`

**Step 1: Create ExchangeSpecificConfig enum**

Add after `Exchange` enum:
```rust
/// Exchange-specific configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ExchangeSpecificConfig {
    Polymarket(PolymarketConfig),
}

impl Default for ExchangeSpecificConfig {
    fn default() -> Self {
        Self::Polymarket(PolymarketConfig::default())
    }
}
```

**Step 2: Update Config struct**

Change:
```rust
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Which exchange to connect to.
    #[serde(default)]
    pub exchange: Exchange,
    /// Polymarket-specific configuration.
    #[serde(default)]
    pub polymarket: PolymarketConfig,
    ...
}
```
To:
```rust
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Which exchange to connect to.
    #[serde(default)]
    pub exchange: Exchange,
    /// Exchange-specific configuration.
    #[serde(default, alias = "polymarket")]
    pub exchange_config: ExchangeSpecificConfig,
    ...
}
```

**Step 3: Update Config::network() method**

Change:
```rust
    pub fn network(&self) -> NetworkConfig {
        match self.exchange {
            Exchange::Polymarket => NetworkConfig {
                environment: self.polymarket.environment,
                ws_url: self.polymarket.ws_url.clone(),
                api_url: self.polymarket.api_url.clone(),
                chain_id: self.polymarket.chain_id,
            },
        }
    }
```
To:
```rust
    pub fn network(&self) -> NetworkConfig {
        match &self.exchange_config {
            ExchangeSpecificConfig::Polymarket(poly) => NetworkConfig {
                environment: poly.environment,
                ws_url: poly.ws_url.clone(),
                api_url: poly.api_url.clone(),
                chain_id: poly.chain_id,
            },
        }
    }
```

**Step 4: Update Config::default()**

Change:
```rust
impl Default for Config {
    fn default() -> Self {
        Self {
            exchange: Exchange::default(),
            polymarket: PolymarketConfig::default(),
            ...
        }
    }
}
```
To:
```rust
impl Default for Config {
    fn default() -> Self {
        Self {
            exchange: Exchange::default(),
            exchange_config: ExchangeSpecificConfig::default(),
            ...
        }
    }
}
```

**Step 5: Add helper method for Polymarket config**

Add to `impl Config`:
```rust
    /// Get Polymarket-specific config if this is a Polymarket exchange.
    #[must_use]
    pub fn polymarket_config(&self) -> Option<&PolymarketConfig> {
        match &self.exchange_config {
            ExchangeSpecificConfig::Polymarket(config) => Some(config),
        }
    }
```

**Step 6: Update any code using config.polymarket directly**

Search for `config.polymarket` usages and update to use `config.polymarket_config().unwrap()` or pattern match.

**Step 7: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 8: Commit**

```bash
git add src/app/config.rs
git commit -m "refactor(config): organize exchange-specific settings under enum"
```

---

### Task 5: Final Verification and Documentation

**Files:**
- Run full test suite
- Verify docs build

**Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings

**Step 3: Build docs**

Run: `cargo doc --no-deps`
Expected: Docs build without warnings

**Step 4: Verify no Polymarket leakage in generic code**

Run: `grep -r "Polymarket" src/core/domain/ src/core/strategy/ src/core/solver/`
Expected: No output (no Polymarket references in generic modules)

**Step 5: Verify no hardcoded outcome names**

Run: `grep -rn '"Yes"\|"No"' src/core/strategy/`
Expected: Only in test code, not in production logic

**Step 6: Final commit (if any fixes needed)**

```bash
git add -A
git commit -m "docs: update docstrings for exchange abstraction cleanup"
```

---

## Summary of Changes

| File | Change |
|------|--------|
| `polymarket/client.rs` | `Client` → `PolymarketClient` |
| `polymarket/websocket.rs` | `DataStream` → `PolymarketDataStream`, `WebSocketHandler` → `PolymarketWebSocketHandler` |
| `polymarket/executor.rs` | `Executor` → `PolymarketExecutor` |
| `polymarket/mod.rs` | Update re-exports |
| `exchange/factory.rs` | Add `create_arbitrage_executor()`, update type names |
| `app/orchestrator.rs` | Use `dyn ArbitrageExecutor` trait |
| `app/config.rs` | Add `ExchangeSpecificConfig` enum |
| `strategy/single_condition.rs` | Use indices instead of "Yes"/"No" |
