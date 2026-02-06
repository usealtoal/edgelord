# Repo Quality Cleanup Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Align licensing and documentation rules with the codebase, refactor `mod.rs` files to be re-export-only, and expand config validation coverage with tests.

**Architecture:** Keep module boundaries intact; move `mod.rs` logic into dedicated modules with re-exports only. Centralize config validation in `Config::validate()` and add integration tests in `tests/config_tests.rs`.

**Tech Stack:** Rust 2021, Cargo, `thiserror`, `serde`, `toml`, `rust_decimal`, `tokio`.

**Skills:** @superpowers:executing-plans

### Task 0: Update SLOC Limit to 500

**Files:**
- Modify: `ARCHITECTURE.md`

**Step 1: Update SLOC limit**

Change:
```
- **Hard limit: 400 SLOC (source lines of code)**
```
To:
```
- **Hard limit: 500 SLOC (source lines of code)**
```

**Step 2: Verification**

No tests required (doc-only change).

**Step 3: Commit**

```bash
git add ARCHITECTURE.md
git commit -m "docs(architecture): raise sloc limit to 500"
```

### Task 1: Align License Metadata With README (Proprietary)

**Files:**
- Create: `LICENSE`
- Modify: `Cargo.toml`

**Step 1: Update crate license metadata**

Edit `Cargo.toml`:
```toml
[package]
name = "edgelord"
version = "0.1.0"
edition = "2021"
description = "Polymarket arbitrage detection and execution"
license = "UNLICENSED"
```

**Step 2: Add proprietary LICENSE file**

Create `LICENSE`:
```text
Copyright (c) 2026 Altoal
All rights reserved.

This software is proprietary and confidential. Unauthorized copying, modification,
or distribution of this software, via any medium, is strictly prohibited.
```

**Step 3: Verification**

No tests required (metadata-only change).

**Step 4: Commit**

```bash
git add Cargo.toml LICENSE
git commit -m "chore(license): align license metadata"
```

> If MIT is intended instead, replace `UNLICENSED` with `MIT`, add the standard MIT license text to `LICENSE`, and update the README badge/text accordingly.

### Task 2: Fix Telegram Doc Path in Config Example

**Files:**
- Modify: `config.toml.example`

**Step 1: Update the comment path**

Change:
```
# See doc/deployment/telegram.md for setup instructions.
```
To:
```
# See docs/deployment/telegram.md for setup instructions.
```

**Step 2: Verification**

No tests required (comment-only change).

**Step 3: Commit**

```bash
git add config.toml.example
git commit -m "docs(config): fix telegram docs path"
```

### Task 3: Refactor `mod.rs` To Re-export Only (Exchange + Strategy)

**Files:**
- Create: `src/core/exchange/types.rs`
- Modify: `src/core/exchange/mod.rs`
- Create: `src/core/strategy/traits.rs`
- Create: `src/core/strategy/registry.rs`
- Modify: `src/core/strategy/mod.rs`

**Step 1: Move exchange types/traits into `types.rs`**

Create `src/core/exchange/types.rs` by moving the following items verbatim out of `src/core/exchange/mod.rs` (no logic changes):
- `ExecutionResult`, `OrderRequest`, `OrderSide`
- `MarketInfo`, `OutcomeInfo`
- `MarketEvent`
- `OrderExecutor`, `MarketFetcher`, `MarketDataStream`, `ArbitrageExecutor`
- `impl MarketDataStream for Box<dyn MarketDataStream>`

Use this file structure (content is the moved code):
```rust
use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::core::domain::{ArbitrageExecutionResult, Opportunity, OrderBook, OrderId, TokenId};
use crate::error::Error;

// <paste the moved structs/enums/traits/impls here, unchanged>
```

Then update `src/core/exchange/mod.rs` to re-export only:
```rust
//! Exchange abstraction layer.
//!
//! ## Adding a New Exchange
//! ... (keep existing module docs)

mod approval;
mod dedup;
mod factory;
mod filter;
pub mod polymarket;
mod reconnecting;
mod scorer;
mod traits;
mod types;

pub use approval::{ApprovalResult, ApprovalStatus, TokenApproval};
pub use dedup::{DedupConfig, DedupStrategy, MessageDeduplicator};
pub use factory::ExchangeFactory;
pub use filter::{MarketFilter, MarketFilterConfig};
pub use reconnecting::ReconnectingDataStream;
pub use scorer::MarketScorer;
pub use traits::ExchangeConfig;
pub use types::{
    ArbitrageExecutor, ExecutionResult, MarketDataStream, MarketEvent, MarketFetcher, MarketInfo,
    OrderExecutor, OrderRequest, OrderSide, OutcomeInfo,
};
```

**Step 2: Split strategy traits/registry into dedicated modules**

Create `src/core/strategy/traits.rs`:
```rust
use crate::core::domain::Opportunity;
use crate::core::strategy::context::{DetectionContext, DetectionResult, MarketContext};

/// A detection strategy that finds arbitrage opportunities.
///
/// Strategies encapsulate specific detection algorithms. Each strategy
/// can be configured independently and may apply to different market types.
pub trait Strategy: Send + Sync {
    /// Unique identifier for this strategy.
    ///
    /// Used in configuration and logging.
    fn name(&self) -> &'static str;

    /// Check if this strategy should run for a given market context.
    ///
    /// For example, single-condition only applies to binary markets,
    /// while market rebalancing applies to multi-outcome markets.
    fn applies_to(&self, ctx: &MarketContext) -> bool;

    /// Detect opportunities given current market state.
    ///
    /// Returns all found opportunities (may be empty).
    fn detect(&self, ctx: &DetectionContext) -> Vec<Opportunity>;

    /// Optional: provide warm-start hint from previous detection.
    ///
    /// Strategies can use this to speed up iterative algorithms
    /// (e.g., Frank-Wolfe can reuse previous solution).
    fn warm_start(&mut self, _previous: &DetectionResult) {}
}
```

Create `src/core/strategy/registry.rs` using the existing implementations and tests from `src/core/strategy/mod.rs` (move them verbatim):
```rust
use std::sync::Arc;

use crate::core::cache::ClusterCache;
use crate::core::domain::Opportunity;
use crate::core::strategy::combinatorial::{CombinatorialConfig, CombinatorialStrategy};
use crate::core::strategy::condition::{SingleConditionConfig, SingleConditionStrategy};
use crate::core::strategy::rebalancing::{
    MarketRebalancingConfig, MarketRebalancingStrategy,
};
use crate::core::strategy::traits::Strategy;
use crate::core::strategy::context::DetectionContext;

/// Registry of enabled strategies.
#[derive(Default)]
pub struct StrategyRegistry {
    strategies: Vec<Box<dyn Strategy>>,
}

impl StrategyRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn builder() -> StrategyRegistryBuilder {
        StrategyRegistryBuilder::new()
    }

    pub fn register(&mut self, strategy: Box<dyn Strategy>) {
        self.strategies.push(strategy);
    }

    #[must_use]
    pub fn strategies(&self) -> &[Box<dyn Strategy>] {
        &self.strategies
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.strategies.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.strategies.is_empty()
    }

    #[must_use]
    pub fn detect_all(&self, ctx: &DetectionContext) -> Vec<Opportunity> {
        let market_ctx = ctx.market_context();
        self.strategies
            .iter()
            .filter(|s| s.applies_to(&market_ctx))
            .flat_map(|s| s.detect(ctx))
            .collect()
    }
}

/// Builder for constructing a [`StrategyRegistry`] from configuration.
#[derive(Default)]
pub struct StrategyRegistryBuilder {
    cluster_cache: Option<Arc<ClusterCache>>,
    single_condition: Option<SingleConditionConfig>,
    market_rebalancing: Option<MarketRebalancingConfig>,
    combinatorial: Option<CombinatorialConfig>,
}

impl StrategyRegistryBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn cluster_cache(mut self, cache: Arc<ClusterCache>) -> Self {
        self.cluster_cache = Some(cache);
        self
    }

    #[must_use]
    pub fn single_condition(mut self, config: SingleConditionConfig) -> Self {
        self.single_condition = Some(config);
        self
    }

    #[must_use]
    pub fn market_rebalancing(mut self, config: MarketRebalancingConfig) -> Self {
        self.market_rebalancing = Some(config);
        self
    }

    #[must_use]
    pub fn combinatorial(mut self, config: CombinatorialConfig) -> Self {
        self.combinatorial = Some(config);
        self
    }

    #[must_use]
    pub fn build(self) -> StrategyRegistry {
        let mut registry = StrategyRegistry::new();

        if let Some(config) = self.single_condition {
            registry.register(Box::new(SingleConditionStrategy::new(config)));
        }

        if let Some(config) = self.market_rebalancing {
            registry.register(Box::new(MarketRebalancingStrategy::new(config)));
        }

        if let Some(config) = self.combinatorial {
            if config.enabled {
                let mut strategy = CombinatorialStrategy::new(config);
                if let Some(cache) = self.cluster_cache {
                    strategy.set_cache(cache);
                }
                registry.register(Box::new(strategy));
            }
        }

        registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::strategy::context::{DetectionContext, MarketContext};

    struct MockStrategy {
        name: &'static str,
        applies: bool,
    }

    impl Strategy for MockStrategy {
        fn name(&self) -> &'static str {
            self.name
        }

        fn applies_to(&self, _ctx: &MarketContext) -> bool {
            self.applies
        }

        fn detect(&self, _ctx: &DetectionContext) -> Vec<Opportunity> {
            vec![]
        }
    }

    #[test]
    fn test_registry_new() {
        let registry = StrategyRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_register() {
        let mut registry = StrategyRegistry::new();
        registry.register(Box::new(MockStrategy {
            name: "test",
            applies: true,
        }));

        assert_eq!(registry.len(), 1);
        assert_eq!(registry.strategies()[0].name(), "test");
    }
}
```

Update `src/core/strategy/mod.rs` to re-export only:
```rust
//! Strategy abstraction for arbitrage detection.
//!
//! (keep existing module docs)

pub mod combinatorial;
mod context;
pub mod rebalancing;
pub mod condition;
mod registry;
mod traits;

pub use combinatorial::{CombinatorialConfig, CombinatorialStrategy};
pub use context::{DetectionContext, DetectionResult, MarketContext};
pub use rebalancing::{
    MarketRebalancingConfig, MarketRebalancingStrategy, RebalancingLeg, RebalancingOpportunity,
};
pub use condition::{SingleConditionConfig, SingleConditionStrategy};
pub use registry::{StrategyRegistry, StrategyRegistryBuilder};
pub use traits::Strategy;
```

**Step 3: Verification**

Run:
```bash
cargo test
```
Expected: PASS (same test count as baseline).

**Step 4: Commit**

```bash
git add src/core/exchange/mod.rs src/core/exchange/types.rs \
  src/core/strategy/mod.rs src/core/strategy/traits.rs src/core/strategy/registry.rs
git commit -m "refactor(core): make mod.rs re-export only"
```

### Task 4: Expand Config Validation + Tests

**Files:**
- Modify: `src/app/config/mod.rs`
- Modify: `tests/config_tests.rs`

**Step 1: Add failing tests**

Append to `tests/config_tests.rs`:
```rust
#[test]
fn config_rejects_negative_risk_limits() {
    let toml = r#"
exchange = "polymarket"

[exchange_config]
type = "polymarket"
ws_url = "wss://ws-subscriptions-clob.polymarket.com/ws/market"
api_url = "https://clob.polymarket.com"

[logging]
level = "info"
format = "pretty"

[risk]
max_position_per_market = -1
"#;

    let path = write_temp_config(toml);
    let result = Config::load(&path);
    let _ = fs::remove_file(&path);

    match result {
        Err(Error::Config(ConfigError::InvalidValue { field: "max_position_per_market", .. })) => {}
        Err(err) => panic!("Expected invalid risk limit error, got {err}"),
        Ok(_) => panic!("Expected invalid risk limit to be rejected"),
    }
}

#[test]
fn config_rejects_invalid_reconnection_backoff() {
    let toml = r#"
exchange = "polymarket"

[exchange_config]
type = "polymarket"
ws_url = "wss://ws-subscriptions-clob.polymarket.com/ws/market"
api_url = "https://clob.polymarket.com"

[logging]
level = "info"
format = "pretty"

[reconnection]
backoff_multiplier = 0.5
"#;

    let path = write_temp_config(toml);
    let result = Config::load(&path);
    let _ = fs::remove_file(&path);

    match result {
        Err(Error::Config(ConfigError::InvalidValue { field: "backoff_multiplier", .. })) => {}
        Err(err) => panic!("Expected invalid backoff error, got {err}"),
        Ok(_) => panic!("Expected invalid backoff to be rejected"),
    }
}

#[test]
fn config_rejects_invalid_latency_targets() {
    let toml = r#"
exchange = "polymarket"

[exchange_config]
type = "polymarket"
ws_url = "wss://ws-subscriptions-clob.polymarket.com/ws/market"
api_url = "https://clob.polymarket.com"

[logging]
level = "info"
format = "pretty"

[governor.latency]
target_p50_ms = 100
target_p95_ms = 50
"#;

    let path = write_temp_config(toml);
    let result = Config::load(&path);
    let _ = fs::remove_file(&path);

    match result {
        Err(Error::Config(ConfigError::InvalidValue { field: "latency_targets", .. })) => {}
        Err(err) => panic!("Expected invalid latency targets error, got {err}"),
        Ok(_) => panic!("Expected invalid latency targets to be rejected"),
    }
}

#[test]
fn config_rejects_invalid_cluster_min_gap() {
    let toml = r#"
exchange = "polymarket"

[exchange_config]
type = "polymarket"
ws_url = "wss://ws-subscriptions-clob.polymarket.com/ws/market"
api_url = "https://clob.polymarket.com"

[logging]
level = "info"
format = "pretty"

[cluster_detection]
enabled = true
min_gap = 1.5
"#;

    let path = write_temp_config(toml);
    let result = Config::load(&path);
    let _ = fs::remove_file(&path);

    match result {
        Err(Error::Config(ConfigError::InvalidValue { field: "min_gap", .. })) => {}
        Err(err) => panic!("Expected invalid min_gap error, got {err}"),
        Ok(_) => panic!("Expected invalid min_gap to be rejected"),
    }
}
```

**Step 2: Run tests to see failures**

Run:
```bash
cargo test tests/config_tests.rs
```
Expected: FAIL with `InvalidValue` mismatches (validation not yet added).

**Step 3: Implement validation**

Update `src/app/config/mod.rs` inside `Config::validate()`:
```rust
        if self.risk.max_position_per_market <= Decimal::ZERO {
            return Err(ConfigError::InvalidValue {
                field: "max_position_per_market",
                reason: "must be greater than 0".to_string(),
            }
            .into());
        }
        if self.risk.max_total_exposure <= Decimal::ZERO {
            return Err(ConfigError::InvalidValue {
                field: "max_total_exposure",
                reason: "must be greater than 0".to_string(),
            }
            .into());
        }
        if self.risk.min_profit_threshold < Decimal::ZERO {
            return Err(ConfigError::InvalidValue {
                field: "min_profit_threshold",
                reason: "must be 0 or greater".to_string(),
            }
            .into());
        }

        if self.reconnection.initial_delay_ms == 0 {
            return Err(ConfigError::InvalidValue {
                field: "initial_delay_ms",
                reason: "must be greater than 0".to_string(),
            }
            .into());
        }
        if self.reconnection.max_delay_ms < self.reconnection.initial_delay_ms {
            return Err(ConfigError::InvalidValue {
                field: "max_delay_ms",
                reason: "must be >= initial_delay_ms".to_string(),
            }
            .into());
        }
        if self.reconnection.backoff_multiplier < 1.0 {
            return Err(ConfigError::InvalidValue {
                field: "backoff_multiplier",
                reason: "must be >= 1.0".to_string(),
            }
            .into());
        }
        if self.reconnection.max_consecutive_failures == 0 {
            return Err(ConfigError::InvalidValue {
                field: "max_consecutive_failures",
                reason: "must be greater than 0".to_string(),
            }
            .into());
        }
        if self.reconnection.circuit_breaker_cooldown_ms == 0 {
            return Err(ConfigError::InvalidValue {
                field: "circuit_breaker_cooldown_ms",
                reason: "must be greater than 0".to_string(),
            }
            .into());
        }

        let latency = &self.governor.latency;
        if latency.target_p50_ms == 0
            || latency.target_p95_ms == 0
            || latency.target_p99_ms == 0
            || latency.max_p99_ms == 0
        {
            return Err(ConfigError::InvalidValue {
                field: "latency_targets",
                reason: "latency targets must be greater than 0".to_string(),
            }
            .into());
        }
        if !(latency.target_p50_ms <= latency.target_p95_ms
            && latency.target_p95_ms <= latency.target_p99_ms
            && latency.target_p99_ms <= latency.max_p99_ms)
        {
            return Err(ConfigError::InvalidValue {
                field: "latency_targets",
                reason: "targets must be ordered p50 <= p95 <= p99 <= max_p99".to_string(),
            }
            .into());
        }

        let scaling = &self.governor.scaling;
        if scaling.check_interval_secs == 0
            || scaling.expand_step == 0
            || scaling.contract_step == 0
            || scaling.cooldown_secs == 0
        {
            return Err(ConfigError::InvalidValue {
                field: "scaling_config",
                reason: "interval/steps/cooldown must be greater than 0".to_string(),
            }
            .into());
        }
        if scaling.expand_threshold <= 0.0 || scaling.contract_threshold <= 0.0 {
            return Err(ConfigError::InvalidValue {
                field: "scaling_config",
                reason: "thresholds must be greater than 0".to_string(),
            }
            .into());
        }

        if self.cluster_detection.enabled {
            if self.cluster_detection.debounce_ms == 0 {
                return Err(ConfigError::InvalidValue {
                    field: "debounce_ms",
                    reason: "must be greater than 0".to_string(),
                }
                .into());
            }
            if self.cluster_detection.min_gap < Decimal::ZERO
                || self.cluster_detection.min_gap > Decimal::ONE
            {
                return Err(ConfigError::InvalidValue {
                    field: "min_gap",
                    reason: "must be between 0 and 1".to_string(),
                }
                .into());
            }
            if self.cluster_detection.max_clusters_per_cycle == 0 {
                return Err(ConfigError::InvalidValue {
                    field: "max_clusters_per_cycle",
                    reason: "must be greater than 0".to_string(),
                }
                .into());
            }
            if self.cluster_detection.channel_capacity == 0 {
                return Err(ConfigError::InvalidValue {
                    field: "channel_capacity",
                    reason: "must be greater than 0".to_string(),
                }
                .into());
            }
        }
```

**Step 4: Re-run tests**

Run:
```bash
cargo test tests/config_tests.rs
```
Expected: PASS.

**Step 5: Commit**

```bash
git add src/app/config/mod.rs tests/config_tests.rs
git commit -m "fix(config): validate governor, reconnection, risk, cluster"
```

---

Plan complete and saved to `docs/plans/2026-02-06-repo-quality-cleanup.md`. Two execution options:

1. Subagent-Driven (this session) - I dispatch a fresh subagent per task, review between tasks, fast iteration
2. Parallel Session (separate) - Open new session with executing-plans, batch execution with checkpoints

Which approach?
