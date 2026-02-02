# Multi-Strategy Architecture with Frank-Wolfe + ILP

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Transform edgelord into a modular, multi-strategy arbitrage system supporting single-condition, market rebalancing, and combinatorial (Frank-Wolfe + ILP) detection strategies with configurable solver backends.

**Architecture:** Strategy trait abstraction with pluggable detectors. Each strategy implements a common interface. Solver abstraction allows swapping HiGHS/Gurobi. Configuration-driven strategy selection and tuning.

**Tech Stack:** Rust 2021, good_lp with HiGHS backend, trait-based strategy pattern, TOML configuration.

---

## Target Structure

```
src/
├── domain/
│   ├── strategy/                    # Strategy abstraction layer
│   │   ├── mod.rs                   # Strategy trait + StrategyRegistry
│   │   ├── context.rs               # DetectionContext, MarketContext
│   │   ├── single_condition.rs      # Refactored from detector.rs
│   │   ├── market_rebalancing.rs    # Sum of all outcomes < $1
│   │   └── combinatorial/           # Frank-Wolfe + ILP
│   │       ├── mod.rs               # CombinatorialStrategy
│   │       ├── frank_wolfe.rs       # FW algorithm implementation
│   │       ├── bregman.rs           # Bregman divergence calculations
│   │       └── constraints.rs       # ILP constraint builder
│   │
│   ├── solver/                      # Solver abstraction layer
│   │   ├── mod.rs                   # Solver trait + SolverConfig
│   │   └── highs.rs                 # HiGHS implementation via good_lp
│   │
│   └── ... (existing modules)
│
├── config.rs                        # Extended with strategies config
└── app.rs                           # Updated to use StrategyRegistry
```

---

## Task 1: Add Solver Dependencies

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add good_lp with HiGHS feature**

Add to `[dependencies]`:

```toml
# Linear programming solver
good_lp = { version = "1.8", default-features = false, features = ["highs"] }
```

**Step 2: Verify dependency resolves**

Run: `cargo check`
Expected: Compiles (HiGHS downloads and builds)

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "deps: add good_lp with HiGHS solver for ILP support"
```

---

## Task 2: Create Solver Abstraction

**Files:**
- Create: `src/domain/solver/mod.rs`
- Create: `src/domain/solver/highs.rs`

**Step 1: Create solver/mod.rs with trait definition**

```rust
//! Solver abstraction for linear and integer programming.

mod highs;

pub use highs::HiGHSSolver;

use crate::error::Result;
use rust_decimal::Decimal;

/// A linear/integer programming solver.
pub trait Solver: Send + Sync {
    /// Solver name for logging/config.
    fn name(&self) -> &'static str;

    /// Solve: minimize c·x subject to Ax >= b, x in bounds.
    /// Returns the optimal x vector and objective value.
    fn solve_lp(&self, problem: &LpProblem) -> Result<LpSolution>;

    /// Solve with integer constraints on specified variables.
    fn solve_ilp(&self, problem: &IlpProblem) -> Result<LpSolution>;
}

/// Linear programming problem definition.
#[derive(Debug, Clone)]
pub struct LpProblem {
    /// Objective coefficients (minimize c·x).
    pub objective: Vec<Decimal>,
    /// Constraint matrix A (row-major).
    pub constraints: Vec<Constraint>,
    /// Variable bounds.
    pub bounds: Vec<VariableBounds>,
}

/// Integer linear programming problem.
#[derive(Debug, Clone)]
pub struct IlpProblem {
    /// Base LP problem.
    pub lp: LpProblem,
    /// Indices of variables that must be integer (binary if bounds are 0-1).
    pub integer_vars: Vec<usize>,
}

/// A single constraint: sum(coeffs[i] * x[i]) >= rhs (or <=, =).
#[derive(Debug, Clone)]
pub struct Constraint {
    pub coefficients: Vec<Decimal>,
    pub sense: ConstraintSense,
    pub rhs: Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintSense {
    GreaterEqual,
    LessEqual,
    Equal,
}

/// Bounds on a variable.
#[derive(Debug, Clone, Copy)]
pub struct VariableBounds {
    pub lower: Option<Decimal>,
    pub upper: Option<Decimal>,
}

impl Default for VariableBounds {
    fn default() -> Self {
        Self { lower: Some(Decimal::ZERO), upper: None }
    }
}

impl VariableBounds {
    pub fn binary() -> Self {
        Self { lower: Some(Decimal::ZERO), upper: Some(Decimal::ONE) }
    }

    pub fn free() -> Self {
        Self { lower: None, upper: None }
    }
}

/// Solution to an LP/ILP problem.
#[derive(Debug, Clone)]
pub struct LpSolution {
    /// Optimal variable values.
    pub values: Vec<Decimal>,
    /// Optimal objective value.
    pub objective: Decimal,
    /// Solver status.
    pub status: SolutionStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolutionStatus {
    Optimal,
    Infeasible,
    Unbounded,
    Error,
}
```

**Step 2: Create solver/highs.rs with HiGHS implementation**

```rust
//! HiGHS solver implementation via good_lp.

use good_lp::{constraint, default_solver, variable, variables, Expression, Solution, SolverModel};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use super::{
    Constraint, ConstraintSense, IlpProblem, LpProblem, LpSolution, SolutionStatus, Solver,
    VariableBounds,
};
use crate::error::{Error, Result};

/// HiGHS-based solver using good_lp.
#[derive(Debug, Default)]
pub struct HiGHSSolver;

impl HiGHSSolver {
    pub fn new() -> Self {
        Self
    }
}

impl Solver for HiGHSSolver {
    fn name(&self) -> &'static str {
        "highs"
    }

    fn solve_lp(&self, problem: &LpProblem) -> Result<LpSolution> {
        solve_with_good_lp(problem, &[])
    }

    fn solve_ilp(&self, problem: &IlpProblem) -> Result<LpSolution> {
        solve_with_good_lp(&problem.lp, &problem.integer_vars)
    }
}

fn solve_with_good_lp(problem: &LpProblem, integer_vars: &[usize]) -> Result<LpSolution> {
    use good_lp::solvers::highs::highs;

    let n = problem.objective.len();
    if n == 0 {
        return Ok(LpSolution {
            values: vec![],
            objective: Decimal::ZERO,
            status: SolutionStatus::Optimal,
        });
    }

    // Create variables
    let mut vars = variables!();
    let mut var_list = Vec::with_capacity(n);

    for (i, bounds) in problem.bounds.iter().enumerate() {
        let mut v = variable();
        if let Some(lb) = bounds.lower {
            v = v.min(lb.to_f64().unwrap_or(0.0));
        }
        if let Some(ub) = bounds.upper {
            v = v.max(ub.to_f64().unwrap_or(f64::INFINITY));
        }
        // Mark as integer if needed
        if integer_vars.contains(&i) {
            v = v.integer();
        }
        var_list.push(vars.add(v));
    }

    // Build objective
    let obj_coeffs: Vec<f64> = problem
        .objective
        .iter()
        .map(|d| d.to_f64().unwrap_or(0.0))
        .collect();

    let objective: Expression = var_list
        .iter()
        .zip(obj_coeffs.iter())
        .map(|(v, c)| *c * *v)
        .sum();

    // Start model
    let mut model = vars.minimise(objective).using(highs);

    // Add constraints
    for constr in &problem.constraints {
        let lhs: Expression = var_list
            .iter()
            .zip(constr.coefficients.iter())
            .map(|(v, c)| c.to_f64().unwrap_or(0.0) * *v)
            .sum();

        let rhs = constr.rhs.to_f64().unwrap_or(0.0);

        match constr.sense {
            ConstraintSense::GreaterEqual => {
                model = model.with(constraint!(lhs >= rhs));
            }
            ConstraintSense::LessEqual => {
                model = model.with(constraint!(lhs <= rhs));
            }
            ConstraintSense::Equal => {
                model = model.with(constraint!(lhs == rhs));
            }
        }
    }

    // Solve
    match model.solve() {
        Ok(solution) => {
            let values: Vec<Decimal> = var_list
                .iter()
                .map(|v| Decimal::try_from(solution.value(*v)).unwrap_or(Decimal::ZERO))
                .collect();

            let obj_value = Decimal::try_from(solution.eval(objective))
                .unwrap_or(Decimal::ZERO);

            Ok(LpSolution {
                values,
                objective: obj_value,
                status: SolutionStatus::Optimal,
            })
        }
        Err(e) => {
            // good_lp returns ResolutionError on infeasible/unbounded
            Ok(LpSolution {
                values: vec![Decimal::ZERO; n],
                objective: Decimal::ZERO,
                status: SolutionStatus::Infeasible,
            })
        }
    }
}
```

**Step 3: Add solver module to domain/mod.rs**

Add to domain/mod.rs:

```rust
pub mod solver;
```

**Step 4: Run cargo check**

Run: `cargo check`
Expected: Compiles

**Step 5: Commit**

```bash
git add src/domain/solver/
git commit -m "feat: add solver abstraction with HiGHS implementation"
```

---

## Task 3: Create Strategy Trait and Context

**Files:**
- Create: `src/domain/strategy/mod.rs`
- Create: `src/domain/strategy/context.rs`

**Step 1: Create strategy/mod.rs with Strategy trait**

```rust
//! Strategy abstraction for arbitrage detection.
//!
//! Strategies implement different detection algorithms:
//! - SingleCondition: YES + NO < $1
//! - MarketRebalancing: Sum of all outcomes < $1
//! - Combinatorial: Frank-Wolfe + ILP for correlated markets

mod context;
mod single_condition;
mod market_rebalancing;
pub mod combinatorial;

pub use context::{DetectionContext, DetectionResult, MarketContext};
pub use single_condition::{SingleConditionStrategy, SingleConditionConfig};
pub use market_rebalancing::{MarketRebalancingStrategy, MarketRebalancingConfig};
pub use combinatorial::{CombinatorialStrategy, CombinatorialConfig};

use crate::domain::Opportunity;

/// A detection strategy that finds arbitrage opportunities.
pub trait Strategy: Send + Sync {
    /// Unique identifier for this strategy.
    fn name(&self) -> &'static str;

    /// Check if this strategy should run for a given market context.
    fn applies_to(&self, ctx: &MarketContext) -> bool;

    /// Detect opportunities given current market state.
    /// Returns all found opportunities (may be empty).
    fn detect(&self, ctx: &DetectionContext) -> Vec<Opportunity>;

    /// Optional: provide warm-start hint from previous detection.
    /// Strategies can use this to speed up iterative algorithms.
    fn warm_start(&mut self, _previous: &DetectionResult) {}
}

/// Registry of enabled strategies.
#[derive(Default)]
pub struct StrategyRegistry {
    strategies: Vec<Box<dyn Strategy>>,
}

impl StrategyRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a strategy.
    pub fn register(&mut self, strategy: Box<dyn Strategy>) {
        self.strategies.push(strategy);
    }

    /// Get all registered strategies.
    pub fn strategies(&self) -> &[Box<dyn Strategy>] {
        &self.strategies
    }

    /// Run all applicable strategies and collect opportunities.
    pub fn detect_all(&self, ctx: &DetectionContext) -> Vec<Opportunity> {
        let market_ctx = ctx.market_context();
        self.strategies
            .iter()
            .filter(|s| s.applies_to(&market_ctx))
            .flat_map(|s| s.detect(ctx))
            .collect()
    }
}
```

**Step 2: Create strategy/context.rs with detection context**

```rust
//! Context types for strategy detection.

use std::sync::Arc;

use crate::domain::{MarketId, MarketPair, Opportunity, OrderBookCache, TokenId};

/// Context for a single market being analyzed.
#[derive(Debug, Clone)]
pub struct MarketContext {
    /// Number of outcomes in the market.
    pub outcome_count: usize,
    /// Whether this market has known dependencies with others.
    pub has_dependencies: bool,
    /// Market IDs of correlated markets (for combinatorial).
    pub correlated_markets: Vec<MarketId>,
}

impl MarketContext {
    /// Simple binary market (YES/NO).
    pub fn binary() -> Self {
        Self {
            outcome_count: 2,
            has_dependencies: false,
            correlated_markets: vec![],
        }
    }

    /// Multi-outcome market.
    pub fn multi_outcome(count: usize) -> Self {
        Self {
            outcome_count: count,
            has_dependencies: false,
            correlated_markets: vec![],
        }
    }
}

/// Full context for detection including market data.
pub struct DetectionContext<'a> {
    /// The market pair being analyzed.
    pub pair: &'a MarketPair,
    /// Order book cache with current prices.
    pub cache: &'a OrderBookCache,
    /// Additional market context.
    pub market_ctx: MarketContext,
}

impl<'a> DetectionContext<'a> {
    pub fn new(pair: &'a MarketPair, cache: &'a OrderBookCache) -> Self {
        Self {
            pair,
            cache,
            market_ctx: MarketContext::binary(),
        }
    }

    pub fn with_market_context(mut self, ctx: MarketContext) -> Self {
        self.market_ctx = ctx;
        self
    }

    pub fn market_context(&self) -> MarketContext {
        self.market_ctx.clone()
    }
}

/// Result from a detection run (for warm-starting).
#[derive(Debug, Clone, Default)]
pub struct DetectionResult {
    /// Opportunities found.
    pub opportunities: Vec<Opportunity>,
    /// Solver state for warm-starting (opaque bytes).
    pub solver_state: Option<Vec<u8>>,
    /// Last computed prices (for delta detection).
    pub last_prices: Vec<(TokenId, rust_decimal::Decimal)>,
}
```

**Step 3: Add strategy module to domain/mod.rs**

Add to domain/mod.rs:

```rust
pub mod strategy;
```

**Step 4: Run cargo check**

Run: `cargo check`
Expected: Compiles (warnings OK for unused)

**Step 5: Commit**

```bash
git add src/domain/strategy/mod.rs src/domain/strategy/context.rs
git commit -m "feat: add Strategy trait and detection context types"
```

---

## Task 4: Refactor Single-Condition as Strategy

**Files:**
- Create: `src/domain/strategy/single_condition.rs`
- Modify: `src/domain/detector.rs` (deprecate, re-export)
- Modify: `src/domain/mod.rs`

**Step 1: Create strategy/single_condition.rs**

```rust
//! Single-condition arbitrage strategy.
//!
//! Detects when YES + NO < $1.00 for binary markets.

use rust_decimal::Decimal;
use serde::Deserialize;

use super::{DetectionContext, MarketContext, Strategy};
use crate::domain::{Opportunity, OrderBookCache, MarketPair};

/// Configuration for single-condition detection.
#[derive(Debug, Clone, Deserialize)]
pub struct SingleConditionConfig {
    /// Minimum edge (profit per $1) to consider.
    #[serde(default = "default_min_edge")]
    pub min_edge: Decimal,

    /// Minimum expected profit in dollars.
    #[serde(default = "default_min_profit")]
    pub min_profit: Decimal,
}

fn default_min_edge() -> Decimal {
    Decimal::new(5, 2) // 0.05
}

fn default_min_profit() -> Decimal {
    Decimal::new(50, 2) // 0.50
}

impl Default for SingleConditionConfig {
    fn default() -> Self {
        Self {
            min_edge: default_min_edge(),
            min_profit: default_min_profit(),
        }
    }
}

/// Single-condition arbitrage detector.
///
/// Finds opportunities where buying YES + NO costs less than $1.00.
pub struct SingleConditionStrategy {
    config: SingleConditionConfig,
}

impl SingleConditionStrategy {
    pub fn new(config: SingleConditionConfig) -> Self {
        Self { config }
    }
}

impl Strategy for SingleConditionStrategy {
    fn name(&self) -> &'static str {
        "single_condition"
    }

    fn applies_to(&self, ctx: &MarketContext) -> bool {
        // Only applies to binary markets
        ctx.outcome_count == 2
    }

    fn detect(&self, ctx: &DetectionContext) -> Vec<Opportunity> {
        detect_single_condition(ctx.pair, ctx.cache, &self.config)
            .into_iter()
            .collect()
    }
}

/// Core detection logic (unchanged from original).
pub fn detect_single_condition(
    pair: &MarketPair,
    cache: &OrderBookCache,
    config: &SingleConditionConfig,
) -> Option<Opportunity> {
    let (yes_book, no_book) = cache.get_pair(pair.yes_token(), pair.no_token());

    let yes_book = yes_book?;
    let no_book = no_book?;

    let yes_ask = yes_book.best_ask()?;
    let no_ask = no_book.best_ask()?;

    let total_cost = yes_ask.price() + no_ask.price();

    if total_cost >= Decimal::ONE {
        return None;
    }

    let edge = Decimal::ONE - total_cost;

    if edge < config.min_edge {
        return None;
    }

    let volume = yes_ask.size().min(no_ask.size());
    let expected_profit = edge * volume;

    if expected_profit < config.min_profit {
        return None;
    }

    Opportunity::builder()
        .market_id(pair.market_id().clone())
        .question(pair.question())
        .yes_token(pair.yes_token().clone(), yes_ask.price())
        .no_token(pair.no_token().clone(), no_ask.price())
        .volume(volume)
        .build()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{MarketId, OrderBook, PriceLevel, TokenId};
    use rust_decimal_macros::dec;

    fn make_pair() -> MarketPair {
        MarketPair::new(
            MarketId::from("test-market"),
            "Test question?",
            TokenId::from("yes-token"),
            TokenId::from("no-token"),
        )
    }

    fn make_config() -> SingleConditionConfig {
        SingleConditionConfig {
            min_edge: dec!(0.05),
            min_profit: dec!(0.50),
        }
    }

    #[test]
    fn test_strategy_name() {
        let strategy = SingleConditionStrategy::new(make_config());
        assert_eq!(strategy.name(), "single_condition");
    }

    #[test]
    fn test_applies_to_binary_only() {
        let strategy = SingleConditionStrategy::new(make_config());
        assert!(strategy.applies_to(&MarketContext::binary()));
        assert!(!strategy.applies_to(&MarketContext::multi_outcome(3)));
    }

    #[test]
    fn test_detects_arbitrage() {
        let pair = make_pair();
        let cache = OrderBookCache::new();
        let config = make_config();

        cache.update(OrderBook::with_levels(
            pair.yes_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.40), dec!(100))],
        ));
        cache.update(OrderBook::with_levels(
            pair.no_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(100))],
        ));

        let opp = detect_single_condition(&pair, &cache, &config);
        assert!(opp.is_some());
        assert_eq!(opp.unwrap().edge(), dec!(0.10));
    }
}
```

**Step 2: Update domain/detector.rs to re-export**

Replace the contents with a re-export for backwards compatibility:

```rust
//! Arbitrage detection (legacy module).
//!
//! This module is deprecated. Use `domain::strategy` instead.
//!
//! Re-exports are provided for backwards compatibility.

pub use crate::domain::strategy::single_condition::{
    detect_single_condition, SingleConditionConfig as DetectorConfig,
};
```

**Step 3: Update domain/mod.rs**

Update exports:

```rust
//! Exchange-agnostic domain logic.

mod ids;
mod market;
mod money;
mod opportunity;
mod orderbook;
mod position;

pub mod solver;
pub mod strategy;

// Legacy re-export for backwards compatibility
mod detector;
pub use detector::{detect_single_condition, DetectorConfig};

// Core domain types
pub use ids::{MarketId, TokenId};
pub use market::{MarketInfo, MarketPair, TokenInfo};
pub use money::{Price, Volume};
pub use opportunity::{Opportunity, OpportunityBuildError, OpportunityBuilder};
pub use position::{Position, PositionId, PositionLeg, PositionStatus, PositionTracker};

// OrderBook types and cache
pub use orderbook::{OrderBook, OrderBookCache, PriceLevel};
```

**Step 4: Run cargo test**

Run: `cargo test single_condition`
Expected: All tests pass

**Step 5: Commit**

```bash
git add src/domain/strategy/single_condition.rs src/domain/detector.rs src/domain/mod.rs
git commit -m "refactor: extract SingleConditionStrategy from detector"
```

---

## Task 5: Implement Market Rebalancing Strategy

**Files:**
- Create: `src/domain/strategy/market_rebalancing.rs`
- Modify: `src/domain/strategy/mod.rs`

**Step 1: Create strategy/market_rebalancing.rs**

```rust
//! Market rebalancing arbitrage strategy.
//!
//! Detects when the sum of all outcome prices < $1.00.
//! This captures 73% of historical arbitrage profits.

use rust_decimal::Decimal;
use serde::Deserialize;

use super::{DetectionContext, MarketContext, Strategy};
use crate::domain::{Opportunity, OrderBookCache, MarketPair, TokenId, Price, Volume};

/// Configuration for market rebalancing detection.
#[derive(Debug, Clone, Deserialize)]
pub struct MarketRebalancingConfig {
    /// Minimum edge (profit per $1) to consider.
    #[serde(default = "default_min_edge")]
    pub min_edge: Decimal,

    /// Minimum expected profit in dollars.
    #[serde(default = "default_min_profit")]
    pub min_profit: Decimal,

    /// Maximum number of outcomes to analyze (skip huge markets).
    #[serde(default = "default_max_outcomes")]
    pub max_outcomes: usize,
}

fn default_min_edge() -> Decimal {
    Decimal::new(3, 2) // 0.03
}

fn default_min_profit() -> Decimal {
    Decimal::ONE // $1.00
}

fn default_max_outcomes() -> usize {
    10
}

impl Default for MarketRebalancingConfig {
    fn default() -> Self {
        Self {
            min_edge: default_min_edge(),
            min_profit: default_min_profit(),
            max_outcomes: default_max_outcomes(),
        }
    }
}

/// Market rebalancing arbitrage detector.
///
/// Finds opportunities where buying all outcomes costs less than $1.00.
/// One outcome must win, so guaranteed $1.00 payout.
pub struct MarketRebalancingStrategy {
    config: MarketRebalancingConfig,
}

impl MarketRebalancingStrategy {
    pub fn new(config: MarketRebalancingConfig) -> Self {
        Self { config }
    }
}

impl Strategy for MarketRebalancingStrategy {
    fn name(&self) -> &'static str {
        "market_rebalancing"
    }

    fn applies_to(&self, ctx: &MarketContext) -> bool {
        // Applies to multi-outcome markets (3+ outcomes)
        // Binary markets are handled by single_condition
        ctx.outcome_count >= 3 && ctx.outcome_count <= self.config.max_outcomes
    }

    fn detect(&self, ctx: &DetectionContext) -> Vec<Opportunity> {
        // Note: This requires multi-outcome market support.
        // For now, we detect on binary pairs but the logic extends.
        // Full implementation needs MarketInfo with all tokens.

        // Binary fallback: treat as single-condition
        if ctx.market_ctx.outcome_count == 2 {
            return vec![];
        }

        // TODO: Implement multi-outcome detection when we have
        // full market info with all token IDs.
        // For now, return empty - this is a placeholder.
        vec![]
    }
}

/// Detect rebalancing opportunity across multiple outcomes.
///
/// # Arguments
/// * `token_ids` - All outcome token IDs for the market
/// * `cache` - Order book cache
/// * `config` - Detection config
///
/// # Returns
/// Opportunity if sum of best asks < $1.00
pub fn detect_rebalancing(
    market_id: &crate::domain::MarketId,
    question: &str,
    token_ids: &[TokenId],
    cache: &OrderBookCache,
    config: &MarketRebalancingConfig,
) -> Option<RebalancingOpportunity> {
    if token_ids.len() < 3 || token_ids.len() > config.max_outcomes {
        return None;
    }

    // Collect best asks for all outcomes
    let mut legs: Vec<RebalancingLeg> = Vec::with_capacity(token_ids.len());
    let mut total_cost = Decimal::ZERO;
    let mut min_volume = Decimal::MAX;

    for token_id in token_ids {
        let book = cache.get(token_id)?;
        let ask = book.best_ask()?;

        total_cost += ask.price();
        min_volume = min_volume.min(ask.size());

        legs.push(RebalancingLeg {
            token_id: token_id.clone(),
            price: ask.price(),
            volume: ask.size(),
        });
    }

    // Check if arbitrage exists
    if total_cost >= Decimal::ONE {
        return None;
    }

    let edge = Decimal::ONE - total_cost;

    if edge < config.min_edge {
        return None;
    }

    let expected_profit = edge * min_volume;

    if expected_profit < config.min_profit {
        return None;
    }

    Some(RebalancingOpportunity {
        market_id: market_id.clone(),
        question: question.to_string(),
        legs,
        total_cost,
        edge,
        volume: min_volume,
        expected_profit,
    })
}

/// A single leg in a rebalancing opportunity.
#[derive(Debug, Clone)]
pub struct RebalancingLeg {
    pub token_id: TokenId,
    pub price: Price,
    pub volume: Volume,
}

/// A market rebalancing opportunity.
#[derive(Debug, Clone)]
pub struct RebalancingOpportunity {
    pub market_id: crate::domain::MarketId,
    pub question: String,
    pub legs: Vec<RebalancingLeg>,
    pub total_cost: Price,
    pub edge: Price,
    pub volume: Volume,
    pub expected_profit: Price,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{MarketId, OrderBook, PriceLevel};
    use rust_decimal_macros::dec;

    #[test]
    fn test_strategy_name() {
        let strategy = MarketRebalancingStrategy::new(MarketRebalancingConfig::default());
        assert_eq!(strategy.name(), "market_rebalancing");
    }

    #[test]
    fn test_applies_to_multi_outcome() {
        let strategy = MarketRebalancingStrategy::new(MarketRebalancingConfig::default());
        assert!(!strategy.applies_to(&MarketContext::binary()));
        assert!(strategy.applies_to(&MarketContext::multi_outcome(3)));
        assert!(strategy.applies_to(&MarketContext::multi_outcome(5)));
    }

    #[test]
    fn test_detect_rebalancing_opportunity() {
        let market_id = MarketId::from("election");
        let tokens = vec![
            TokenId::from("candidate-a"),
            TokenId::from("candidate-b"),
            TokenId::from("candidate-c"),
        ];
        let cache = OrderBookCache::new();
        let config = MarketRebalancingConfig::default();

        // Set up order books: 0.30 + 0.30 + 0.30 = 0.90 < 1.00
        cache.update(OrderBook::with_levels(
            tokens[0].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.30), dec!(100))],
        ));
        cache.update(OrderBook::with_levels(
            tokens[1].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.30), dec!(100))],
        ));
        cache.update(OrderBook::with_levels(
            tokens[2].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.30), dec!(100))],
        ));

        let opp = detect_rebalancing(&market_id, "Who wins?", &tokens, &cache, &config);
        assert!(opp.is_some());

        let opp = opp.unwrap();
        assert_eq!(opp.total_cost, dec!(0.90));
        assert_eq!(opp.edge, dec!(0.10));
        assert_eq!(opp.expected_profit, dec!(10.00));
        assert_eq!(opp.legs.len(), 3);
    }

    #[test]
    fn test_no_opportunity_when_sum_exceeds_one() {
        let market_id = MarketId::from("election");
        let tokens = vec![
            TokenId::from("candidate-a"),
            TokenId::from("candidate-b"),
            TokenId::from("candidate-c"),
        ];
        let cache = OrderBookCache::new();
        let config = MarketRebalancingConfig::default();

        // Set up order books: 0.40 + 0.40 + 0.40 = 1.20 > 1.00
        cache.update(OrderBook::with_levels(
            tokens[0].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.40), dec!(100))],
        ));
        cache.update(OrderBook::with_levels(
            tokens[1].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.40), dec!(100))],
        ));
        cache.update(OrderBook::with_levels(
            tokens[2].clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.40), dec!(100))],
        ));

        let opp = detect_rebalancing(&market_id, "Who wins?", &tokens, &cache, &config);
        assert!(opp.is_none());
    }
}
```

**Step 2: Update strategy/mod.rs exports**

Already included in Task 3, but verify `market_rebalancing` is exported.

**Step 3: Run cargo test**

Run: `cargo test market_rebalancing`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/domain/strategy/market_rebalancing.rs
git commit -m "feat: add MarketRebalancingStrategy for multi-outcome markets"
```

---

## Task 6: Implement Frank-Wolfe Algorithm

**Files:**
- Create: `src/domain/strategy/combinatorial/mod.rs`
- Create: `src/domain/strategy/combinatorial/frank_wolfe.rs`
- Create: `src/domain/strategy/combinatorial/bregman.rs`

**Step 1: Create combinatorial/mod.rs**

```rust
//! Combinatorial arbitrage detection using Frank-Wolfe + ILP.
//!
//! This strategy detects arbitrage opportunities across correlated markets
//! where logical dependencies create exploitable mispricings.

mod frank_wolfe;
mod bregman;

pub use frank_wolfe::{FrankWolfe, FrankWolfeConfig};
pub use bregman::{bregman_divergence, bregman_gradient};

use rust_decimal::Decimal;
use serde::Deserialize;

use super::{DetectionContext, MarketContext, Strategy};
use crate::domain::Opportunity;

/// Configuration for combinatorial strategy.
#[derive(Debug, Clone, Deserialize)]
pub struct CombinatorialConfig {
    /// Maximum Frank-Wolfe iterations per detection.
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,

    /// Convergence tolerance (stop when gap < this).
    #[serde(default = "default_tolerance")]
    pub tolerance: Decimal,

    /// Minimum arbitrage gap to act on.
    #[serde(default = "default_gap_threshold")]
    pub gap_threshold: Decimal,

    /// Enable this strategy.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_max_iterations() -> usize {
    20
}

fn default_tolerance() -> Decimal {
    Decimal::new(1, 4) // 0.0001
}

fn default_gap_threshold() -> Decimal {
    Decimal::new(2, 2) // 0.02
}

fn default_enabled() -> bool {
    false // Disabled by default until configured
}

impl Default for CombinatorialConfig {
    fn default() -> Self {
        Self {
            max_iterations: default_max_iterations(),
            tolerance: default_tolerance(),
            gap_threshold: default_gap_threshold(),
            enabled: default_enabled(),
        }
    }
}

/// Combinatorial arbitrage strategy using Frank-Wolfe + ILP.
pub struct CombinatorialStrategy {
    config: CombinatorialConfig,
    fw: FrankWolfe,
}

impl CombinatorialStrategy {
    pub fn new(config: CombinatorialConfig) -> Self {
        let fw_config = FrankWolfeConfig {
            max_iterations: config.max_iterations,
            tolerance: config.tolerance,
        };
        Self {
            config,
            fw: FrankWolfe::new(fw_config),
        }
    }
}

impl Strategy for CombinatorialStrategy {
    fn name(&self) -> &'static str {
        "combinatorial"
    }

    fn applies_to(&self, ctx: &MarketContext) -> bool {
        // Only applies to markets with known dependencies
        self.config.enabled && ctx.has_dependencies
    }

    fn detect(&self, ctx: &DetectionContext) -> Vec<Opportunity> {
        // Full implementation requires:
        // 1. Market dependency graph
        // 2. ILP constraint builder
        // 3. Frank-Wolfe projection
        //
        // This is a complex feature that needs:
        // - Dependency detection (potentially LLM-assisted)
        // - Constraint encoding
        // - Multi-market state aggregation
        //
        // For now, return empty. Full implementation in future tasks.
        vec![]
    }
}
```

**Step 2: Create combinatorial/frank_wolfe.rs**

```rust
//! Frank-Wolfe algorithm for Bregman projection.
//!
//! The Frank-Wolfe (conditional gradient) algorithm solves:
//!   min_{μ ∈ M} D(μ || θ)
//!
//! where D is the Bregman divergence and M is the marginal polytope.
//!
//! Instead of full projection, it uses a linear minimization oracle (ILP)
//! to iteratively improve the solution.

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use crate::domain::solver::{IlpProblem, LpProblem, LpSolution, Solver, SolutionStatus};
use crate::error::Result;

use super::bregman::{bregman_divergence, bregman_gradient};

/// Configuration for Frank-Wolfe algorithm.
#[derive(Debug, Clone)]
pub struct FrankWolfeConfig {
    /// Maximum iterations.
    pub max_iterations: usize,
    /// Convergence tolerance.
    pub tolerance: Decimal,
}

impl Default for FrankWolfeConfig {
    fn default() -> Self {
        Self {
            max_iterations: 20,
            tolerance: Decimal::new(1, 4),
        }
    }
}

/// Frank-Wolfe algorithm state.
pub struct FrankWolfe {
    config: FrankWolfeConfig,
}

impl FrankWolfe {
    pub fn new(config: FrankWolfeConfig) -> Self {
        Self { config }
    }

    /// Run Frank-Wolfe projection.
    ///
    /// # Arguments
    /// * `theta` - Current market prices (may be outside M)
    /// * `ilp_problem` - ILP defining the feasible set M
    /// * `solver` - ILP solver to use
    ///
    /// # Returns
    /// * `FrankWolfeResult` with projected prices and arbitrage gap
    pub fn project<S: Solver>(
        &self,
        theta: &[Decimal],
        ilp_problem: &IlpProblem,
        solver: &S,
    ) -> Result<FrankWolfeResult> {
        let n = theta.len();
        if n == 0 {
            return Ok(FrankWolfeResult {
                mu: vec![],
                gap: Decimal::ZERO,
                iterations: 0,
                converged: true,
            });
        }

        // Initialize mu at theta (or feasible point if theta infeasible)
        let mut mu = theta.to_vec();
        let mut iterations = 0;
        let mut gap = Decimal::MAX;

        for _ in 0..self.config.max_iterations {
            iterations += 1;

            // Compute gradient of Bregman divergence at mu
            let grad = bregman_gradient(&mu, theta);

            // Solve linear minimization oracle: min_{s ∈ M} <grad, s>
            let oracle_problem = IlpProblem {
                lp: LpProblem {
                    objective: grad.clone(),
                    constraints: ilp_problem.lp.constraints.clone(),
                    bounds: ilp_problem.lp.bounds.clone(),
                },
                integer_vars: ilp_problem.integer_vars.clone(),
            };

            let solution = solver.solve_ilp(&oracle_problem)?;

            if solution.status != SolutionStatus::Optimal {
                break;
            }

            let s = &solution.values;

            // Compute Frank-Wolfe gap: <grad, mu - s>
            gap = grad
                .iter()
                .zip(mu.iter())
                .zip(s.iter())
                .map(|((g, m), si)| *g * (*m - *si))
                .sum();

            // Check convergence
            if gap.abs() < self.config.tolerance {
                break;
            }

            // Line search: find optimal step size gamma
            // For LMSR, closed-form gamma is complex; use simple 2/(t+2) schedule
            let gamma = Decimal::TWO / Decimal::from(iterations + 2);

            // Update: mu = (1 - gamma) * mu + gamma * s
            let one_minus_gamma = Decimal::ONE - gamma;
            for i in 0..n {
                mu[i] = one_minus_gamma * mu[i] + gamma * s[i];
            }
        }

        // Compute final divergence (arbitrage profit potential)
        let divergence = bregman_divergence(&mu, theta);

        Ok(FrankWolfeResult {
            mu,
            gap: divergence,
            iterations,
            converged: gap.abs() < self.config.tolerance,
        })
    }
}

/// Result of Frank-Wolfe projection.
#[derive(Debug, Clone)]
pub struct FrankWolfeResult {
    /// Projected prices (on or near the marginal polytope).
    pub mu: Vec<Decimal>,
    /// Final gap (approximates arbitrage profit).
    pub gap: Decimal,
    /// Number of iterations run.
    pub iterations: usize,
    /// Whether algorithm converged.
    pub converged: bool,
}

impl FrankWolfeResult {
    /// Check if significant arbitrage exists.
    pub fn has_arbitrage(&self, threshold: Decimal) -> bool {
        self.gap > threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::solver::{Constraint, ConstraintSense, HiGHSSolver, VariableBounds};
    use rust_decimal_macros::dec;

    #[test]
    fn test_frank_wolfe_simple() {
        let config = FrankWolfeConfig {
            max_iterations: 10,
            tolerance: dec!(0.001),
        };
        let fw = FrankWolfe::new(config);
        let solver = HiGHSSolver::new();

        // Simple 2-outcome market: probabilities must sum to 1
        // theta = [0.3, 0.3] sums to 0.6 (arbitrage!)
        let theta = vec![dec!(0.3), dec!(0.3)];

        // ILP: x1 + x2 = 1, x in [0,1]
        let ilp = IlpProblem {
            lp: LpProblem {
                objective: vec![Decimal::ZERO; 2], // Will be replaced by gradient
                constraints: vec![Constraint {
                    coefficients: vec![Decimal::ONE, Decimal::ONE],
                    sense: ConstraintSense::Equal,
                    rhs: Decimal::ONE,
                }],
                bounds: vec![VariableBounds::binary(); 2],
            },
            integer_vars: vec![], // LP relaxation for this test
        };

        let result = fw.project(&theta, &ilp, &solver).unwrap();

        // Projected prices should sum closer to 1
        let sum: Decimal = result.mu.iter().sum();
        assert!(sum > dec!(0.9), "Sum should be close to 1, got {}", sum);
    }
}
```

**Step 3: Create combinatorial/bregman.rs**

```rust
//! Bregman divergence calculations for LMSR cost function.
//!
//! For the Logarithmic Market Scoring Rule (LMSR):
//! - Cost function: C(q) = b * log(Σ exp(qᵢ/b))
//! - Conjugate: R(μ) = Σ μᵢ * ln(μᵢ) (negative entropy)
//! - Bregman divergence: D(μ||θ) = KL divergence
//!
//! The divergence D(μ*||θ) equals the maximum arbitrage profit.

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

/// Compute Bregman divergence D(μ||θ) for LMSR.
///
/// This is the KL divergence when R is negative entropy.
/// D(μ||θ) = Σ μᵢ * ln(μᵢ/θᵢ)
///
/// # Arguments
/// * `mu` - Target probability vector (must be valid distribution)
/// * `theta` - Current market prices
pub fn bregman_divergence(mu: &[Decimal], theta: &[Decimal]) -> Decimal {
    if mu.len() != theta.len() || mu.is_empty() {
        return Decimal::ZERO;
    }

    let mut divergence = Decimal::ZERO;
    let epsilon = Decimal::new(1, 10); // 1e-10 for numerical stability

    for (m, t) in mu.iter().zip(theta.iter()) {
        if *m > epsilon && *t > epsilon {
            // μ * ln(μ/θ) = μ * (ln(μ) - ln(θ))
            let m_f64 = m.to_f64().unwrap_or(0.0);
            let t_f64 = t.to_f64().unwrap_or(1.0);

            if m_f64 > 0.0 && t_f64 > 0.0 {
                let term = m_f64 * (m_f64.ln() - t_f64.ln());
                divergence += Decimal::try_from(term).unwrap_or(Decimal::ZERO);
            }
        }
    }

    divergence
}

/// Compute gradient of Bregman divergence ∇D(μ||θ) w.r.t. μ.
///
/// For KL divergence: ∂D/∂μᵢ = ln(μᵢ/θᵢ) + 1
///
/// # Arguments
/// * `mu` - Current iterate
/// * `theta` - Target (fixed)
pub fn bregman_gradient(mu: &[Decimal], theta: &[Decimal]) -> Vec<Decimal> {
    let epsilon = Decimal::new(1, 10);

    mu.iter()
        .zip(theta.iter())
        .map(|(m, t)| {
            let m_safe = (*m).max(epsilon);
            let t_safe = (*t).max(epsilon);

            let m_f64 = m_safe.to_f64().unwrap_or(1.0);
            let t_f64 = t_safe.to_f64().unwrap_or(1.0);

            // ln(μ/θ) + 1 = ln(μ) - ln(θ) + 1
            let grad = m_f64.ln() - t_f64.ln() + 1.0;
            Decimal::try_from(grad).unwrap_or(Decimal::ZERO)
        })
        .collect()
}

/// Compute the LMSR cost function C(q).
///
/// C(q) = b * ln(Σ exp(qᵢ/b))
///
/// # Arguments
/// * `q` - Quantity vector
/// * `b` - Liquidity parameter
pub fn lmsr_cost(q: &[Decimal], b: Decimal) -> Decimal {
    if q.is_empty() || b == Decimal::ZERO {
        return Decimal::ZERO;
    }

    let b_f64 = b.to_f64().unwrap_or(1.0);

    let sum_exp: f64 = q
        .iter()
        .map(|qi| {
            let qi_f64 = qi.to_f64().unwrap_or(0.0);
            (qi_f64 / b_f64).exp()
        })
        .sum();

    let cost = b_f64 * sum_exp.ln();
    Decimal::try_from(cost).unwrap_or(Decimal::ZERO)
}

/// Compute LMSR prices from quantities.
///
/// Pᵢ = exp(qᵢ/b) / Σₖ exp(qₖ/b)
pub fn lmsr_prices(q: &[Decimal], b: Decimal) -> Vec<Decimal> {
    if q.is_empty() || b == Decimal::ZERO {
        return vec![];
    }

    let b_f64 = b.to_f64().unwrap_or(1.0);

    let exps: Vec<f64> = q
        .iter()
        .map(|qi| {
            let qi_f64 = qi.to_f64().unwrap_or(0.0);
            (qi_f64 / b_f64).exp()
        })
        .collect();

    let sum_exp: f64 = exps.iter().sum();

    exps.iter()
        .map(|e| Decimal::try_from(e / sum_exp).unwrap_or(Decimal::ZERO))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_bregman_divergence_same_distribution() {
        let mu = vec![dec!(0.5), dec!(0.5)];
        let theta = vec![dec!(0.5), dec!(0.5)];

        let d = bregman_divergence(&mu, &theta);
        assert!(d.abs() < dec!(0.001), "Same distribution should have ~0 divergence");
    }

    #[test]
    fn test_bregman_divergence_different() {
        let mu = vec![dec!(0.7), dec!(0.3)];
        let theta = vec![dec!(0.5), dec!(0.5)];

        let d = bregman_divergence(&mu, &theta);
        assert!(d > Decimal::ZERO, "Different distributions should have positive divergence");
    }

    #[test]
    fn test_lmsr_prices_sum_to_one() {
        let q = vec![dec!(1), dec!(2), dec!(3)];
        let b = dec!(1);

        let prices = lmsr_prices(&q, b);
        let sum: Decimal = prices.iter().sum();

        assert!(
            (sum - Decimal::ONE).abs() < dec!(0.001),
            "LMSR prices should sum to 1, got {}",
            sum
        );
    }

    #[test]
    fn test_lmsr_prices_equal_quantities() {
        let q = vec![dec!(1), dec!(1), dec!(1)];
        let b = dec!(1);

        let prices = lmsr_prices(&q, b);

        // Equal quantities should give equal prices
        let expected = Decimal::ONE / Decimal::from(3);
        for p in &prices {
            assert!(
                (*p - expected).abs() < dec!(0.01),
                "Equal quantities should give ~1/3 prices"
            );
        }
    }
}
```

**Step 4: Run cargo test**

Run: `cargo test combinatorial`
Expected: All tests pass

**Step 5: Commit**

```bash
git add src/domain/strategy/combinatorial/
git commit -m "feat: add Frank-Wolfe algorithm with Bregman divergence"
```

---

## Task 7: Update Configuration System

**Files:**
- Modify: `src/config.rs`
- Modify: `config.toml`

**Step 1: Add strategy configuration to Config**

Update `src/config.rs`:

```rust
use serde::Deserialize;
use std::path::Path;
use tracing_subscriber::{fmt, EnvFilter};

use crate::domain::strategy::{
    SingleConditionConfig, MarketRebalancingConfig, CombinatorialConfig,
};
use crate::error::{ConfigError, Result};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub network: NetworkConfig,
    pub logging: LoggingConfig,
    #[serde(default)]
    pub strategies: StrategiesConfig,
    #[serde(default)]
    pub wallet: WalletConfig,
}

/// Configuration for all detection strategies.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct StrategiesConfig {
    /// Enabled strategy names.
    #[serde(default = "default_enabled_strategies")]
    pub enabled: Vec<String>,

    /// Single-condition strategy config.
    #[serde(default)]
    pub single_condition: SingleConditionConfig,

    /// Market rebalancing strategy config.
    #[serde(default)]
    pub market_rebalancing: MarketRebalancingConfig,

    /// Combinatorial (Frank-Wolfe + ILP) strategy config.
    #[serde(default)]
    pub combinatorial: CombinatorialConfig,
}

fn default_enabled_strategies() -> Vec<String> {
    vec!["single_condition".to_string()]
}

// ... rest of config.rs unchanged ...
```

**Step 2: Update config.toml with strategy sections**

```toml
[network]
ws_url = "wss://ws-subscriptions-clob.polymarket.com/ws/market"
api_url = "https://clob.polymarket.com"
chain_id = 80002  # Amoy testnet

[logging]
level = "info"
format = "pretty"

[strategies]
enabled = ["single_condition", "market_rebalancing"]

[strategies.single_condition]
min_edge = 0.05
min_profit = 0.50

[strategies.market_rebalancing]
min_edge = 0.03
min_profit = 1.00
max_outcomes = 10

[strategies.combinatorial]
enabled = false
max_iterations = 20
tolerance = 0.0001
gap_threshold = 0.02

[wallet]
# Private key loaded from WALLET_PRIVATE_KEY env var (never commit!)
```

**Step 3: Run cargo check**

Run: `cargo check`
Expected: Compiles

**Step 4: Commit**

```bash
git add src/config.rs config.toml
git commit -m "config: add multi-strategy configuration system"
```

---

## Task 8: Update App to Use Strategy Registry

**Files:**
- Modify: `src/app.rs`

**Step 1: Update app.rs to use StrategyRegistry**

```rust
//! App orchestration module.

use std::sync::Arc;

use crate::config::Config;
use crate::domain::strategy::{
    DetectionContext, MarketContext, SingleConditionStrategy, MarketRebalancingStrategy,
    CombinatorialStrategy, Strategy, StrategyRegistry,
};
use crate::domain::{Opportunity, OrderBookCache};
use crate::error::Result;
use crate::polymarket::{
    MarketRegistry, PolymarketClient, PolymarketExecutor, WebSocketHandler, WsMessage,
};
use tracing::{debug, error, info, warn};

/// Main application struct.
pub struct App;

impl App {
    /// Run the main application loop.
    pub async fn run(config: Config) -> Result<()> {
        let executor = init_executor(&config).await;

        // Build strategy registry from config
        let strategies = build_strategy_registry(&config);
        info!(
            strategies = ?strategies.strategies().iter().map(|s| s.name()).collect::<Vec<_>>(),
            "Strategies loaded"
        );

        let client = PolymarketClient::new(config.network.api_url.clone());
        let markets = client.get_active_markets(20).await?;

        if markets.is_empty() {
            warn!("No active markets found");
            return Ok(());
        }

        let registry = MarketRegistry::from_markets(&markets);

        info!(
            total_markets = markets.len(),
            yes_no_pairs = registry.len(),
            "Markets loaded"
        );

        if registry.is_empty() {
            warn!("No YES/NO market pairs found");
            return Ok(());
        }

        for pair in registry.pairs() {
            debug!(
                market_id = %pair.market_id(),
                question = %pair.question(),
                "Tracking market"
            );
        }

        let token_ids: Vec<String> = registry
            .pairs()
            .iter()
            .flat_map(|p| vec![p.yes_token().to_string(), p.no_token().to_string()])
            .collect();

        info!(tokens = token_ids.len(), "Subscribing to tokens");

        let cache = Arc::new(OrderBookCache::new());
        let registry = Arc::new(registry);
        let strategies = Arc::new(strategies);

        let handler = WebSocketHandler::new(config.network.ws_url);

        let cache_clone = cache.clone();
        let registry_clone = registry.clone();
        let strategies_clone = strategies.clone();
        let executor_clone = executor.clone();

        handler
            .run(token_ids, move |msg| {
                handle_message(
                    msg,
                    &cache_clone,
                    &registry_clone,
                    &strategies_clone,
                    executor_clone.clone(),
                );
            })
            .await?;

        Ok(())
    }
}

/// Build strategy registry from configuration.
fn build_strategy_registry(config: &Config) -> StrategyRegistry {
    let mut registry = StrategyRegistry::new();

    for name in &config.strategies.enabled {
        match name.as_str() {
            "single_condition" => {
                registry.register(Box::new(SingleConditionStrategy::new(
                    config.strategies.single_condition.clone(),
                )));
            }
            "market_rebalancing" => {
                registry.register(Box::new(MarketRebalancingStrategy::new(
                    config.strategies.market_rebalancing.clone(),
                )));
            }
            "combinatorial" => {
                if config.strategies.combinatorial.enabled {
                    registry.register(Box::new(CombinatorialStrategy::new(
                        config.strategies.combinatorial.clone(),
                    )));
                }
            }
            unknown => {
                warn!(strategy = unknown, "Unknown strategy in config, skipping");
            }
        }
    }

    registry
}

/// Initialize the executor if wallet is configured.
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

/// Handle incoming WebSocket messages.
fn handle_message(
    msg: WsMessage,
    cache: &OrderBookCache,
    registry: &MarketRegistry,
    strategies: &StrategyRegistry,
    executor: Option<Arc<PolymarketExecutor>>,
) {
    match msg {
        WsMessage::Book(book) => {
            let orderbook = book.to_orderbook();
            let token_id = orderbook.token_id().clone();
            cache.update(orderbook);

            if let Some(pair) = registry.get_market_for_token(&token_id) {
                // Create detection context
                let ctx = DetectionContext::new(pair, cache);

                // Run all applicable strategies
                let opportunities = strategies.detect_all(&ctx);

                for opp in opportunities {
                    log_opportunity(&opp);

                    if let Some(exec) = executor.clone() {
                        spawn_execution(exec, opp);
                    }
                }
            }
        }
        WsMessage::PriceChange(_) => {}
        _ => {}
    }
}

/// Log detected opportunity.
fn log_opportunity(opp: &Opportunity) {
    info!(
        market = %opp.market_id(),
        question = %opp.question(),
        yes_ask = %opp.yes_ask(),
        no_ask = %opp.no_ask(),
        total_cost = %opp.total_cost(),
        edge = %opp.edge(),
        volume = %opp.volume(),
        expected_profit = %opp.expected_profit(),
        "ARBITRAGE DETECTED"
    );
}

/// Spawn async execution without blocking message processing.
fn spawn_execution(executor: Arc<PolymarketExecutor>, opportunity: Opportunity) {
    tokio::spawn(async move {
        match executor.execute_arbitrage(&opportunity).await {
            Ok(result) => {
                info!(result = ?result, "Execution completed");
            }
            Err(e) => {
                error!(error = %e, "Execution failed");
            }
        }
    });
}
```

**Step 2: Run cargo check**

Run: `cargo check`
Expected: Compiles

**Step 3: Run cargo test**

Run: `cargo test`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/app.rs
git commit -m "refactor: update app to use StrategyRegistry"
```

---

## Task 9: Add Integration Tests

**Files:**
- Create: `tests/strategy_tests.rs`

**Step 1: Create comprehensive strategy tests**

```rust
//! Integration tests for strategy system.

use edgelord::domain::strategy::{
    DetectionContext, MarketContext, SingleConditionConfig, SingleConditionStrategy,
    MarketRebalancingConfig, Strategy, StrategyRegistry,
};
use edgelord::domain::{MarketId, MarketPair, OrderBook, OrderBookCache, PriceLevel, TokenId};
use rust_decimal_macros::dec;

fn make_pair() -> MarketPair {
    MarketPair::new(
        MarketId::from("test-market"),
        "Will it happen?",
        TokenId::from("yes-token"),
        TokenId::from("no-token"),
    )
}

fn setup_arbitrage_books(cache: &OrderBookCache, pair: &MarketPair) {
    // YES: 0.40, NO: 0.50 = 0.90 total (10% edge)
    cache.update(OrderBook::with_levels(
        pair.yes_token().clone(),
        vec![],
        vec![PriceLevel::new(dec!(0.40), dec!(100))],
    ));
    cache.update(OrderBook::with_levels(
        pair.no_token().clone(),
        vec![],
        vec![PriceLevel::new(dec!(0.50), dec!(100))],
    ));
}

fn setup_no_arbitrage_books(cache: &OrderBookCache, pair: &MarketPair) {
    // YES: 0.50, NO: 0.50 = 1.00 total (no edge)
    cache.update(OrderBook::with_levels(
        pair.yes_token().clone(),
        vec![],
        vec![PriceLevel::new(dec!(0.50), dec!(100))],
    ));
    cache.update(OrderBook::with_levels(
        pair.no_token().clone(),
        vec![],
        vec![PriceLevel::new(dec!(0.50), dec!(100))],
    ));
}

#[test]
fn test_strategy_registry_detects_with_single_condition() {
    let mut registry = StrategyRegistry::new();
    registry.register(Box::new(SingleConditionStrategy::new(
        SingleConditionConfig::default(),
    )));

    let pair = make_pair();
    let cache = OrderBookCache::new();
    setup_arbitrage_books(&cache, &pair);

    let ctx = DetectionContext::new(&pair, &cache);
    let opportunities = registry.detect_all(&ctx);

    assert_eq!(opportunities.len(), 1);
    assert_eq!(opportunities[0].edge(), dec!(0.10));
}

#[test]
fn test_strategy_registry_empty_when_no_arbitrage() {
    let mut registry = StrategyRegistry::new();
    registry.register(Box::new(SingleConditionStrategy::new(
        SingleConditionConfig::default(),
    )));

    let pair = make_pair();
    let cache = OrderBookCache::new();
    setup_no_arbitrage_books(&cache, &pair);

    let ctx = DetectionContext::new(&pair, &cache);
    let opportunities = registry.detect_all(&ctx);

    assert!(opportunities.is_empty());
}

#[test]
fn test_multiple_strategies_in_registry() {
    let mut registry = StrategyRegistry::new();

    registry.register(Box::new(SingleConditionStrategy::new(
        SingleConditionConfig::default(),
    )));
    // MarketRebalancing won't trigger on binary markets
    registry.register(Box::new(edgelord::domain::strategy::MarketRebalancingStrategy::new(
        MarketRebalancingConfig::default(),
    )));

    assert_eq!(registry.strategies().len(), 2);

    let pair = make_pair();
    let cache = OrderBookCache::new();
    setup_arbitrage_books(&cache, &pair);

    let ctx = DetectionContext::new(&pair, &cache);
    let opportunities = registry.detect_all(&ctx);

    // Only single_condition should fire (binary market)
    assert_eq!(opportunities.len(), 1);
}

#[test]
fn test_strategy_applies_to_filtering() {
    let single = SingleConditionStrategy::new(SingleConditionConfig::default());

    assert!(single.applies_to(&MarketContext::binary()));
    assert!(!single.applies_to(&MarketContext::multi_outcome(3)));
}
```

**Step 2: Run tests**

Run: `cargo test strategy`
Expected: All tests pass

**Step 3: Commit**

```bash
git add tests/strategy_tests.rs
git commit -m "test: add strategy system integration tests"
```

---

## Task 10: Documentation and Cleanup

**Files:**
- Update: `src/domain/mod.rs` (doc comments)
- Update: `src/lib.rs` (doc comments)

**Step 1: Add comprehensive documentation**

Update lib.rs crate docs:

```rust
//! Edgelord - Multi-strategy arbitrage detection and execution.
//!
//! # Architecture
//!
//! - `domain::strategy` - Pluggable detection strategies
//!   - `SingleConditionStrategy` - YES + NO < $1 (26.7% of historical profits)
//!   - `MarketRebalancingStrategy` - Sum of outcomes < $1 (73.1% of profits)
//!   - `CombinatorialStrategy` - Frank-Wolfe + ILP for correlated markets
//!
//! - `domain::solver` - LP/ILP solver abstraction
//!   - `HiGHSSolver` - Open-source HiGHS via good_lp
//!
//! - `exchange` - Exchange abstraction layer
//! - `polymarket` - Polymarket implementation
//!
//! # Example
//!
//! ```no_run
//! use edgelord::config::Config;
//! use edgelord::domain::strategy::{SingleConditionStrategy, StrategyRegistry};
//!
//! let mut registry = StrategyRegistry::new();
//! registry.register(Box::new(SingleConditionStrategy::new(Default::default())));
//! ```

#[cfg(feature = "polymarket")]
pub mod app;
pub mod config;
pub mod domain;
pub mod error;
pub mod exchange;
#[cfg(feature = "polymarket")]
pub mod polymarket;
```

**Step 2: Run cargo doc**

Run: `cargo doc --no-deps --open`
Expected: Documentation builds and looks good

**Step 3: Run full test suite**

Run: `cargo test`
Expected: All tests pass

**Step 4: Run clippy**

Run: `cargo clippy --all-features`
Expected: No warnings

**Step 5: Final commit**

```bash
git add src/lib.rs src/domain/mod.rs
git commit -m "docs: add comprehensive documentation for strategy system"
```

---

## Verification Checklist

Before marking complete:

- [ ] `cargo check --all-features` passes
- [ ] `cargo test` passes (all tests)
- [ ] `cargo clippy --all-features` passes
- [ ] `cargo doc --no-deps` builds without warnings
- [ ] Config file has strategy sections
- [ ] Single-condition strategy works as before
- [ ] Market rebalancing strategy has tests passing
- [ ] Frank-Wolfe + Bregman divergence has tests passing
- [ ] HiGHS solver integration works
- [ ] StrategyRegistry correctly dispatches to strategies

---

## Summary of Changes

| Component | Description |
|-----------|-------------|
| `domain/solver/` | Solver abstraction with HiGHS implementation |
| `domain/strategy/` | Strategy trait and registry |
| `strategy/single_condition.rs` | Refactored from detector.rs |
| `strategy/market_rebalancing.rs` | New: multi-outcome detection |
| `strategy/combinatorial/` | Frank-Wolfe + Bregman + ILP |
| `config.rs` | Extended with strategy configuration |
| `app.rs` | Uses StrategyRegistry for detection |

## Future Work

1. **Dependency Detection** - LLM-assisted market correlation discovery
2. **Constraint Builder** - Encode logical dependencies as ILP constraints
3. **Multi-Market State** - Aggregate order books across correlated markets
4. **Warm-Starting** - Reuse solver state across detection cycles
5. **Gurobi Backend** - Optional high-performance solver for heavy workloads
