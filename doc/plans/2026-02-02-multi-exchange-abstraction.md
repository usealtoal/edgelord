# Multi-Exchange Abstraction Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Refactor the codebase to support multiple prediction market exchanges by abstracting away Polymarket-specific assumptions about market structure, pricing, and execution.

**Architecture:** Create exchange-agnostic domain types that support N-outcome markets with configurable payout amounts. Replace hardcoded `Decimal::ONE` with market-provided payout values. Make the registry generic enough to handle any outcome naming convention. Keep Polymarket as the only implementation but ensure the abstractions allow adding new exchanges without touching core logic.

**Tech Stack:** Rust, rust_decimal, serde, async_trait

---

## Overview

The codebase currently has these Polymarket-specific assumptions baked in:

1. **Binary markets only** - `MarketPair` has `yes_token`/`no_token` fields
2. **$1.00 payout assumption** - `Decimal::ONE` hardcoded in edge calculations
3. **YES/NO outcome naming** - Registry looks for outcomes named "yes"/"no"
4. **Opportunity struct** - Tied to binary YES/NO structure

This plan creates generic abstractions while keeping Polymarket working.

---

### Task 1: Create Generic Market Structure

**Files:**
- Create: `src/core/domain/generic_market.rs`
- Modify: `src/core/domain/mod.rs`
- Test: Tests in `src/core/domain/generic_market.rs`

**Context:** The current `MarketPair` struct only supports 2 outcomes (YES/NO). We need a generic `Market` struct that supports N outcomes with arbitrary names.

**Step 1: Write the failing test**

Add to new file `src/core/domain/generic_market.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_market_creation_binary() {
        let market = Market::new(
            MarketId::from("market-1"),
            "Will it rain?",
            vec![
                Outcome::new(TokenId::from("yes-token"), "Yes"),
                Outcome::new(TokenId::from("no-token"), "No"),
            ],
            dec!(1.00),
        );

        assert_eq!(market.market_id().as_str(), "market-1");
        assert_eq!(market.question(), "Will it rain?");
        assert_eq!(market.outcomes().len(), 2);
        assert_eq!(market.payout(), dec!(1.00));
        assert!(market.is_binary());
    }

    #[test]
    fn test_market_creation_multi_outcome() {
        let market = Market::new(
            MarketId::from("market-2"),
            "Who wins?",
            vec![
                Outcome::new(TokenId::from("trump"), "Trump"),
                Outcome::new(TokenId::from("biden"), "Biden"),
                Outcome::new(TokenId::from("other"), "Other"),
            ],
            dec!(1.00),
        );

        assert!(!market.is_binary());
        assert_eq!(market.outcomes().len(), 3);
    }

    #[test]
    fn test_outcome_accessors() {
        let outcome = Outcome::new(TokenId::from("token-1"), "Yes");

        assert_eq!(outcome.token_id().as_str(), "token-1");
        assert_eq!(outcome.name(), "Yes");
    }

    #[test]
    fn test_market_get_outcome_by_name() {
        let market = Market::new(
            MarketId::from("m1"),
            "Q?",
            vec![
                Outcome::new(TokenId::from("yes"), "Yes"),
                Outcome::new(TokenId::from("no"), "No"),
            ],
            dec!(1.00),
        );

        assert!(market.outcome_by_name("Yes").is_some());
        assert!(market.outcome_by_name("yes").is_some()); // case-insensitive
        assert!(market.outcome_by_name("No").is_some());
        assert!(market.outcome_by_name("Maybe").is_none());
    }

    #[test]
    fn test_market_token_ids() {
        let market = Market::new(
            MarketId::from("m1"),
            "Q?",
            vec![
                Outcome::new(TokenId::from("t1"), "A"),
                Outcome::new(TokenId::from("t2"), "B"),
            ],
            dec!(1.00),
        );

        let ids: Vec<_> = market.token_ids().collect();
        assert_eq!(ids.len(), 2);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p edgelord --lib generic_market`
Expected: FAIL - module not found

**Step 3: Write minimal implementation**

Create `src/core/domain/generic_market.rs`:

```rust
//! Generic market types supporting N-outcome markets.
//!
//! This module provides exchange-agnostic market representation that works
//! with any number of outcomes and configurable payout amounts.

use rust_decimal::Decimal;

use super::id::{MarketId, TokenId};

/// A single outcome in a market.
#[derive(Debug, Clone)]
pub struct Outcome {
    token_id: TokenId,
    name: String,
}

impl Outcome {
    /// Create a new outcome.
    pub fn new(token_id: TokenId, name: impl Into<String>) -> Self {
        Self {
            token_id,
            name: name.into(),
        }
    }

    /// Get the token ID for this outcome.
    #[must_use]
    pub const fn token_id(&self) -> &TokenId {
        &self.token_id
    }

    /// Get the outcome name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// A generic market with N outcomes.
///
/// Supports any number of outcomes (binary, multi-outcome, etc.)
/// and configurable payout amounts per exchange.
#[derive(Debug, Clone)]
pub struct Market {
    market_id: MarketId,
    question: String,
    outcomes: Vec<Outcome>,
    /// The payout amount for this market (e.g., $1.00 for Polymarket).
    payout: Decimal,
}

impl Market {
    /// Create a new market.
    pub fn new(
        market_id: MarketId,
        question: impl Into<String>,
        outcomes: Vec<Outcome>,
        payout: Decimal,
    ) -> Self {
        Self {
            market_id,
            question: question.into(),
            outcomes,
            payout,
        }
    }

    /// Get the market ID.
    #[must_use]
    pub const fn market_id(&self) -> &MarketId {
        &self.market_id
    }

    /// Get the market question.
    #[must_use]
    pub fn question(&self) -> &str {
        &self.question
    }

    /// Get all outcomes.
    #[must_use]
    pub fn outcomes(&self) -> &[Outcome] {
        &self.outcomes
    }

    /// Get the payout amount for this market.
    #[must_use]
    pub const fn payout(&self) -> Decimal {
        self.payout
    }

    /// Check if this is a binary (2-outcome) market.
    #[must_use]
    pub fn is_binary(&self) -> bool {
        self.outcomes.len() == 2
    }

    /// Get outcome count.
    #[must_use]
    pub fn outcome_count(&self) -> usize {
        self.outcomes.len()
    }

    /// Find an outcome by name (case-insensitive).
    #[must_use]
    pub fn outcome_by_name(&self, name: &str) -> Option<&Outcome> {
        let name_lower = name.to_lowercase();
        self.outcomes
            .iter()
            .find(|o| o.name.to_lowercase() == name_lower)
    }

    /// Get an iterator over all token IDs.
    pub fn token_ids(&self) -> impl Iterator<Item = &TokenId> {
        self.outcomes.iter().map(|o| &o.token_id)
    }
}
```

**Step 4: Update module exports**

Edit `src/core/domain/mod.rs` to add:

```rust
mod generic_market;

pub use generic_market::{Market, Outcome};
```

**Step 5: Run test to verify it passes**

Run: `cargo test -p edgelord --lib generic_market`
Expected: PASS

**Step 6: Commit**

```bash
git add src/core/domain/generic_market.rs src/core/domain/mod.rs
git commit -m "Add generic Market struct supporting N outcomes"
```

---

### Task 2: Create Generic Opportunity Structure

**Files:**
- Create: `src/core/domain/generic_opportunity.rs`
- Modify: `src/core/domain/mod.rs`
- Test: Tests in `src/core/domain/generic_opportunity.rs`

**Context:** The current `Opportunity` struct has `yes_token`/`no_token` fields. We need a generic struct that works with any number of outcomes and uses the market's payout amount instead of hardcoded `Decimal::ONE`.

**Step 1: Write the failing test**

Add to new file `src/core/domain/generic_opportunity.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_generic_opportunity_binary() {
        let legs = vec![
            OpportunityLeg::new(TokenId::from("yes"), dec!(0.40)),
            OpportunityLeg::new(TokenId::from("no"), dec!(0.50)),
        ];

        let opp = GenericOpportunity::new(
            MarketId::from("m1"),
            "Will it rain?",
            legs,
            dec!(100),
            dec!(1.00), // payout
        );

        assert_eq!(opp.market_id().as_str(), "m1");
        assert_eq!(opp.total_cost(), dec!(0.90));
        assert_eq!(opp.edge(), dec!(0.10)); // 1.00 - 0.90
        assert_eq!(opp.expected_profit(), dec!(10.00)); // 0.10 * 100
    }

    #[test]
    fn test_generic_opportunity_different_payout() {
        // Hypothetical exchange with $10 payout
        let legs = vec![
            OpportunityLeg::new(TokenId::from("a"), dec!(4.00)),
            OpportunityLeg::new(TokenId::from("b"), dec!(5.00)),
        ];

        let opp = GenericOpportunity::new(
            MarketId::from("m1"),
            "Q?",
            legs,
            dec!(10),
            dec!(10.00), // $10 payout
        );

        assert_eq!(opp.total_cost(), dec!(9.00));
        assert_eq!(opp.edge(), dec!(1.00)); // 10.00 - 9.00
        assert_eq!(opp.expected_profit(), dec!(10.00)); // 1.00 * 10
    }

    #[test]
    fn test_opportunity_leg_accessors() {
        let leg = OpportunityLeg::new(TokenId::from("t1"), dec!(0.55));

        assert_eq!(leg.token_id().as_str(), "t1");
        assert_eq!(leg.ask_price(), dec!(0.55));
    }

    #[test]
    fn test_generic_opportunity_negative_edge() {
        let legs = vec![
            OpportunityLeg::new(TokenId::from("yes"), dec!(0.60)),
            OpportunityLeg::new(TokenId::from("no"), dec!(0.50)),
        ];

        let opp = GenericOpportunity::new(
            MarketId::from("m1"),
            "Q?",
            legs,
            dec!(100),
            dec!(1.00),
        );

        assert_eq!(opp.total_cost(), dec!(1.10));
        assert_eq!(opp.edge(), dec!(-0.10));
    }

    #[test]
    fn test_generic_opportunity_legs_iter() {
        let legs = vec![
            OpportunityLeg::new(TokenId::from("a"), dec!(0.30)),
            OpportunityLeg::new(TokenId::from("b"), dec!(0.30)),
            OpportunityLeg::new(TokenId::from("c"), dec!(0.30)),
        ];

        let opp = GenericOpportunity::new(
            MarketId::from("m1"),
            "Q?",
            legs,
            dec!(100),
            dec!(1.00),
        );

        assert_eq!(opp.legs().len(), 3);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p edgelord --lib generic_opportunity`
Expected: FAIL - module not found

**Step 3: Write minimal implementation**

Create `src/core/domain/generic_opportunity.rs`:

```rust
//! Generic opportunity type for N-outcome markets.
//!
//! Supports any number of legs and configurable payout amounts.

use rust_decimal::Decimal;

use super::id::{MarketId, TokenId};
use super::money::Price;

/// A single leg (token purchase) in an opportunity.
#[derive(Debug, Clone)]
pub struct OpportunityLeg {
    token_id: TokenId,
    ask_price: Price,
}

impl OpportunityLeg {
    /// Create a new opportunity leg.
    pub fn new(token_id: TokenId, ask_price: Price) -> Self {
        Self { token_id, ask_price }
    }

    /// Get the token ID.
    #[must_use]
    pub const fn token_id(&self) -> &TokenId {
        &self.token_id
    }

    /// Get the ask price.
    #[must_use]
    pub const fn ask_price(&self) -> Price {
        self.ask_price
    }
}

/// A generic arbitrage opportunity supporting N outcomes.
///
/// Unlike the binary-specific `Opportunity`, this supports any number
/// of legs and uses market-provided payout amounts instead of hardcoded $1.00.
#[derive(Debug, Clone)]
pub struct GenericOpportunity {
    market_id: MarketId,
    question: String,
    legs: Vec<OpportunityLeg>,
    volume: Decimal,
    payout: Decimal,
    // Derived fields
    total_cost: Price,
    edge: Price,
    expected_profit: Price,
}

impl GenericOpportunity {
    /// Create a new generic opportunity.
    ///
    /// Automatically calculates derived fields (total_cost, edge, expected_profit).
    pub fn new(
        market_id: MarketId,
        question: impl Into<String>,
        legs: Vec<OpportunityLeg>,
        volume: Decimal,
        payout: Decimal,
    ) -> Self {
        let total_cost: Decimal = legs.iter().map(|l| l.ask_price).sum();
        let edge = payout - total_cost;
        let expected_profit = edge * volume;

        Self {
            market_id,
            question: question.into(),
            legs,
            volume,
            payout,
            total_cost,
            edge,
            expected_profit,
        }
    }

    /// Get the market ID.
    #[must_use]
    pub const fn market_id(&self) -> &MarketId {
        &self.market_id
    }

    /// Get the market question.
    #[must_use]
    pub fn question(&self) -> &str {
        &self.question
    }

    /// Get all legs.
    #[must_use]
    pub fn legs(&self) -> &[OpportunityLeg] {
        &self.legs
    }

    /// Get the volume.
    #[must_use]
    pub const fn volume(&self) -> Decimal {
        self.volume
    }

    /// Get the payout amount.
    #[must_use]
    pub const fn payout(&self) -> Decimal {
        self.payout
    }

    /// Get the total cost (sum of all leg prices).
    #[must_use]
    pub const fn total_cost(&self) -> Price {
        self.total_cost
    }

    /// Get the edge (payout - total_cost).
    #[must_use]
    pub const fn edge(&self) -> Price {
        self.edge
    }

    /// Get the expected profit (edge * volume).
    #[must_use]
    pub const fn expected_profit(&self) -> Price {
        self.expected_profit
    }
}
```

**Step 4: Update module exports**

Edit `src/core/domain/mod.rs` to add:

```rust
mod generic_opportunity;

pub use generic_opportunity::{GenericOpportunity, OpportunityLeg};
```

**Step 5: Run test to verify it passes**

Run: `cargo test -p edgelord --lib generic_opportunity`
Expected: PASS

**Step 6: Commit**

```bash
git add src/core/domain/generic_opportunity.rs src/core/domain/mod.rs
git commit -m "Add GenericOpportunity supporting N outcomes and configurable payout"
```

---

### Task 3: Create Generic Market Registry

**Files:**
- Create: `src/core/domain/market_registry.rs`
- Modify: `src/core/domain/mod.rs`
- Test: Tests in `src/core/domain/market_registry.rs`

**Context:** The current `MarketRegistry` in `polymarket/registry.rs` looks for "yes"/"no" outcomes. We need a generic registry that works with any outcome names and can be configured per exchange.

**Step 1: Write the failing test**

Add to new file `src/core/domain/market_registry.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn make_binary_market(id: &str, yes_id: &str, no_id: &str) -> Market {
        Market::new(
            MarketId::from(id),
            "Q?",
            vec![
                Outcome::new(TokenId::from(yes_id), "Yes"),
                Outcome::new(TokenId::from(no_id), "No"),
            ],
            dec!(1.00),
        )
    }

    fn make_multi_market(id: &str) -> Market {
        Market::new(
            MarketId::from(id),
            "Who wins?",
            vec![
                Outcome::new(TokenId::from("t1"), "Trump"),
                Outcome::new(TokenId::from("t2"), "Biden"),
                Outcome::new(TokenId::from("t3"), "Other"),
            ],
            dec!(1.00),
        )
    }

    #[test]
    fn test_registry_new() {
        let registry = GenericMarketRegistry::new();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_registry_add_market() {
        let mut registry = GenericMarketRegistry::new();
        let market = make_binary_market("m1", "yes", "no");

        registry.add(market);

        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_registry_get_by_token() {
        let mut registry = GenericMarketRegistry::new();
        registry.add(make_binary_market("m1", "yes-1", "no-1"));

        let yes_token = TokenId::from("yes-1");
        let no_token = TokenId::from("no-1");
        let unknown = TokenId::from("unknown");

        assert!(registry.get_by_token(&yes_token).is_some());
        assert!(registry.get_by_token(&no_token).is_some());
        assert!(registry.get_by_token(&unknown).is_none());
    }

    #[test]
    fn test_registry_binary_markets() {
        let mut registry = GenericMarketRegistry::new();
        registry.add(make_binary_market("m1", "y1", "n1"));
        registry.add(make_multi_market("m2"));
        registry.add(make_binary_market("m3", "y3", "n3"));

        let binary: Vec<_> = registry.binary_markets().collect();
        assert_eq!(binary.len(), 2);
    }

    #[test]
    fn test_registry_multi_outcome_markets() {
        let mut registry = GenericMarketRegistry::new();
        registry.add(make_binary_market("m1", "y1", "n1"));
        registry.add(make_multi_market("m2"));

        let multi: Vec<_> = registry.multi_outcome_markets().collect();
        assert_eq!(multi.len(), 1);
    }

    #[test]
    fn test_registry_all_markets() {
        let mut registry = GenericMarketRegistry::new();
        registry.add(make_binary_market("m1", "y1", "n1"));
        registry.add(make_multi_market("m2"));

        assert_eq!(registry.markets().len(), 2);
    }

    #[test]
    fn test_registry_all_token_ids() {
        let mut registry = GenericMarketRegistry::new();
        registry.add(make_binary_market("m1", "y1", "n1"));

        let ids: Vec<_> = registry.all_token_ids().collect();
        assert_eq!(ids.len(), 2);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p edgelord --lib market_registry`
Expected: FAIL - module not found

**Step 3: Write minimal implementation**

Create `src/core/domain/market_registry.rs`:

```rust
//! Generic market registry for any exchange.
//!
//! Maps token IDs to their containing markets, supporting any outcome structure.

use std::collections::HashMap;

use super::generic_market::{Market, Outcome};
use super::id::{MarketId, TokenId};

/// Exchange-agnostic market registry.
///
/// Unlike the Polymarket-specific registry, this doesn't assume YES/NO naming
/// or binary-only markets. Works with any number of outcomes per market.
pub struct GenericMarketRegistry {
    /// Maps token ID -> market
    token_to_market: HashMap<TokenId, Market>,
    /// All registered markets
    markets: Vec<Market>,
}

impl GenericMarketRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            token_to_market: HashMap::new(),
            markets: Vec::new(),
        }
    }

    /// Add a market to the registry.
    ///
    /// Automatically indexes all token IDs for lookup.
    pub fn add(&mut self, market: Market) {
        for outcome in market.outcomes() {
            self.token_to_market
                .insert(outcome.token_id().clone(), market.clone());
        }
        self.markets.push(market);
    }

    /// Look up market by token ID.
    #[must_use]
    pub fn get_by_token(&self, token_id: &TokenId) -> Option<&Market> {
        self.token_to_market.get(token_id)
    }

    /// Get all markets.
    #[must_use]
    pub fn markets(&self) -> &[Market] {
        &self.markets
    }

    /// Get binary (2-outcome) markets only.
    pub fn binary_markets(&self) -> impl Iterator<Item = &Market> {
        self.markets.iter().filter(|m| m.is_binary())
    }

    /// Get multi-outcome (3+) markets only.
    pub fn multi_outcome_markets(&self) -> impl Iterator<Item = &Market> {
        self.markets.iter().filter(|m| !m.is_binary())
    }

    /// Get all token IDs across all markets.
    pub fn all_token_ids(&self) -> impl Iterator<Item = &TokenId> {
        self.markets.iter().flat_map(|m| m.token_ids())
    }

    /// Number of registered markets.
    #[must_use]
    pub fn len(&self) -> usize {
        self.markets.len()
    }

    /// Check if registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.markets.is_empty()
    }
}

impl Default for GenericMarketRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 4: Update module exports**

Edit `src/core/domain/mod.rs` to add:

```rust
mod market_registry;

pub use market_registry::GenericMarketRegistry;
```

**Step 5: Run test to verify it passes**

Run: `cargo test -p edgelord --lib market_registry`
Expected: PASS

**Step 6: Commit**

```bash
git add src/core/domain/market_registry.rs src/core/domain/mod.rs
git commit -m "Add GenericMarketRegistry supporting any outcome structure"
```

---

### Task 4: Add Exchange Trait for Market Configuration

**Files:**
- Create: `src/core/exchange/traits.rs`
- Modify: `src/core/exchange/mod.rs`
- Test: Tests in `src/core/exchange/traits.rs`

**Context:** Different exchanges have different payout amounts, outcome naming conventions, and API structures. We need a trait that exchanges implement to provide these configurations.

**Step 1: Write the failing test**

Add to new file `src/core/exchange/traits.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    struct MockExchange;

    impl ExchangeConfig for MockExchange {
        fn name(&self) -> &'static str {
            "mock"
        }

        fn default_payout(&self) -> Decimal {
            dec!(1.00)
        }

        fn binary_outcome_names(&self) -> (&'static str, &'static str) {
            ("Yes", "No")
        }
    }

    #[test]
    fn test_exchange_config_name() {
        let exchange = MockExchange;
        assert_eq!(exchange.name(), "mock");
    }

    #[test]
    fn test_exchange_config_payout() {
        let exchange = MockExchange;
        assert_eq!(exchange.default_payout(), dec!(1.00));
    }

    #[test]
    fn test_exchange_config_outcome_names() {
        let exchange = MockExchange;
        let (positive, negative) = exchange.binary_outcome_names();
        assert_eq!(positive, "Yes");
        assert_eq!(negative, "No");
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p edgelord --lib exchange::traits`
Expected: FAIL - module not found

**Step 3: Write minimal implementation**

Create `src/core/exchange/traits.rs`:

```rust
//! Exchange configuration traits.
//!
//! Defines what information each exchange must provide for the generic
//! abstractions to work correctly.

use rust_decimal::Decimal;

/// Configuration provided by each exchange implementation.
///
/// This trait captures the exchange-specific details needed for:
/// - Market structure (payout amounts)
/// - Outcome naming conventions (for registry building)
/// - Identification (for logging/debugging)
pub trait ExchangeConfig: Send + Sync {
    /// Exchange name for logging/identification.
    fn name(&self) -> &'static str;

    /// Default payout amount for markets on this exchange.
    ///
    /// For example, Polymarket uses $1.00 per share.
    fn default_payout(&self) -> Decimal;

    /// The outcome names used for binary markets.
    ///
    /// Returns (positive_outcome, negative_outcome).
    /// For Polymarket this is ("Yes", "No").
    fn binary_outcome_names(&self) -> (&'static str, &'static str);

    /// Check if an outcome name matches the "positive" outcome.
    fn is_positive_outcome(&self, name: &str) -> bool {
        let (positive, _) = self.binary_outcome_names();
        name.eq_ignore_ascii_case(positive)
    }

    /// Check if an outcome name matches the "negative" outcome.
    fn is_negative_outcome(&self, name: &str) -> bool {
        let (_, negative) = self.binary_outcome_names();
        name.eq_ignore_ascii_case(negative)
    }
}
```

**Step 4: Update module exports**

Edit `src/core/exchange/mod.rs` to add after existing mod declarations:

```rust
mod traits;

pub use traits::ExchangeConfig;
```

**Step 5: Run test to verify it passes**

Run: `cargo test -p edgelord --lib exchange::traits`
Expected: PASS

**Step 6: Commit**

```bash
git add src/core/exchange/traits.rs src/core/exchange/mod.rs
git commit -m "Add ExchangeConfig trait for exchange-specific configuration"
```

---

### Task 5: Implement ExchangeConfig for Polymarket

**Files:**
- Create: `src/core/exchange/polymarket/config.rs`
- Modify: `src/core/exchange/polymarket/mod.rs`
- Test: Tests in `src/core/exchange/polymarket/config.rs`

**Context:** Polymarket needs to implement the `ExchangeConfig` trait with its specific values ($1.00 payout, "Yes"/"No" outcomes).

**Step 1: Write the failing test**

Add to new file `src/core/exchange/polymarket/config.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_polymarket_name() {
        let config = PolymarketExchangeConfig;
        assert_eq!(config.name(), "polymarket");
    }

    #[test]
    fn test_polymarket_payout() {
        let config = PolymarketExchangeConfig;
        assert_eq!(config.default_payout(), dec!(1.00));
    }

    #[test]
    fn test_polymarket_outcome_names() {
        let config = PolymarketExchangeConfig;
        let (positive, negative) = config.binary_outcome_names();
        assert_eq!(positive, "Yes");
        assert_eq!(negative, "No");
    }

    #[test]
    fn test_polymarket_is_positive_outcome() {
        let config = PolymarketExchangeConfig;

        assert!(config.is_positive_outcome("Yes"));
        assert!(config.is_positive_outcome("yes"));
        assert!(config.is_positive_outcome("YES"));
        assert!(!config.is_positive_outcome("No"));
    }

    #[test]
    fn test_polymarket_is_negative_outcome() {
        let config = PolymarketExchangeConfig;

        assert!(config.is_negative_outcome("No"));
        assert!(config.is_negative_outcome("no"));
        assert!(config.is_negative_outcome("NO"));
        assert!(!config.is_negative_outcome("Yes"));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p edgelord --lib polymarket::config`
Expected: FAIL - module not found

**Step 3: Write minimal implementation**

Create `src/core/exchange/polymarket/config.rs`:

```rust
//! Polymarket exchange configuration.

use rust_decimal::Decimal;

use crate::core::exchange::ExchangeConfig;

/// Polymarket-specific exchange configuration.
///
/// Polymarket uses:
/// - $1.00 payout per share
/// - "Yes"/"No" outcome naming for binary markets
pub struct PolymarketExchangeConfig;

impl ExchangeConfig for PolymarketExchangeConfig {
    fn name(&self) -> &'static str {
        "polymarket"
    }

    fn default_payout(&self) -> Decimal {
        Decimal::ONE
    }

    fn binary_outcome_names(&self) -> (&'static str, &'static str) {
        ("Yes", "No")
    }
}

/// Convenience constant for Polymarket's payout amount.
pub const POLYMARKET_PAYOUT: Decimal = Decimal::ONE;
```

**Step 4: Update module exports**

Edit `src/core/exchange/polymarket/mod.rs` to add:

```rust
mod config;

pub use config::{PolymarketExchangeConfig, POLYMARKET_PAYOUT};
```

**Step 5: Run test to verify it passes**

Run: `cargo test -p edgelord --lib polymarket::config`
Expected: PASS

**Step 6: Commit**

```bash
git add src/core/exchange/polymarket/config.rs src/core/exchange/polymarket/mod.rs
git commit -m "Implement ExchangeConfig for Polymarket"
```

---

### Task 6: Add Payout to Strategy Detection Context

**Files:**
- Modify: `src/core/strategy/context.rs`
- Test: Tests in `src/core/strategy/context.rs`

**Context:** The strategy detection context needs to provide the market's payout amount so strategies can use it instead of hardcoded `Decimal::ONE`.

**Step 1: Read current file**

Read `src/core/strategy/context.rs` to understand current structure.

**Step 2: Write the failing test**

Add test to existing test module in `src/core/strategy/context.rs`:

```rust
#[test]
fn test_detection_context_payout() {
    let pair = make_pair();
    let cache = OrderBookCache::new();

    let ctx = DetectionContext::with_payout(&pair, &cache, dec!(1.00));
    assert_eq!(ctx.payout(), dec!(1.00));

    let ctx = DetectionContext::with_payout(&pair, &cache, dec!(10.00));
    assert_eq!(ctx.payout(), dec!(10.00));
}

#[test]
fn test_detection_context_default_payout() {
    let pair = make_pair();
    let cache = OrderBookCache::new();

    // Default constructor should use 1.00
    let ctx = DetectionContext::new(&pair, &cache);
    assert_eq!(ctx.payout(), dec!(1.00));
}
```

**Step 3: Run test to verify it fails**

Run: `cargo test -p edgelord --lib strategy::context::tests::test_detection_context_payout`
Expected: FAIL - method not found

**Step 4: Implement the changes**

Modify `src/core/strategy/context.rs` to add payout field:

Add to `DetectionContext` struct:
```rust
payout: Decimal,
```

Add constructor:
```rust
/// Create a detection context with explicit payout.
pub fn with_payout(pair: &'a MarketPair, cache: &'a OrderBookCache, payout: Decimal) -> Self {
    Self { pair, cache, payout }
}
```

Update existing `new`:
```rust
/// Create a new detection context (defaults to $1.00 payout).
pub fn new(pair: &'a MarketPair, cache: &'a OrderBookCache) -> Self {
    Self::with_payout(pair, cache, Decimal::ONE)
}
```

Add accessor:
```rust
/// Get the payout amount for this market.
#[must_use]
pub const fn payout(&self) -> Decimal {
    self.payout
}
```

**Step 5: Run test to verify it passes**

Run: `cargo test -p edgelord --lib strategy::context`
Expected: PASS

**Step 6: Commit**

```bash
git add src/core/strategy/context.rs
git commit -m "Add payout amount to DetectionContext"
```

---

### Task 7: Update SingleConditionStrategy to Use Payout

**Files:**
- Modify: `src/core/strategy/single_condition.rs`
- Test: Existing tests + new test

**Context:** Replace hardcoded `Decimal::ONE` with `ctx.payout()` in edge calculation.

**Step 1: Write the failing test**

Add test to `src/core/strategy/single_condition.rs`:

```rust
#[test]
fn test_detects_arbitrage_with_custom_payout() {
    let pair = make_pair();
    let cache = OrderBookCache::new();
    let config = make_config();

    // Prices that work for $10 payout but not $1
    // YES: 4.00, NO: 5.00 = 9.00 total
    cache.update(OrderBook::with_levels(
        pair.yes_token().clone(),
        vec![],
        vec![PriceLevel::new(dec!(4.00), dec!(100))],
    ));
    cache.update(OrderBook::with_levels(
        pair.no_token().clone(),
        vec![],
        vec![PriceLevel::new(dec!(5.00), dec!(100))],
    ));

    // With $1 payout, this is negative edge (would not detect)
    let ctx = DetectionContext::new(&pair, &cache);
    let strategy = SingleConditionStrategy::new(config.clone());
    assert!(strategy.detect(&ctx).is_empty());

    // With $10 payout, edge is 10 - 9 = 1 (10%)
    let ctx = DetectionContext::with_payout(&pair, &cache, dec!(10.00));
    let opportunities = strategy.detect(&ctx);
    assert_eq!(opportunities.len(), 1);
    assert_eq!(opportunities[0].edge(), dec!(1.00));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p edgelord --lib single_condition::tests::test_detects_arbitrage_with_custom_payout`
Expected: FAIL - opportunity has wrong edge (still using Decimal::ONE)

**Step 3: Implement the changes**

Modify `detect_single_condition` in `src/core/strategy/single_condition.rs`:

Change signature to accept payout:
```rust
pub fn detect_single_condition(
    pair: &MarketPair,
    cache: &OrderBookCache,
    config: &SingleConditionConfig,
    payout: Decimal,
) -> Option<Opportunity> {
```

Replace `Decimal::ONE` with `payout`:
```rust
if total_cost >= payout {
    return None;
}

let edge = payout - total_cost;
```

Update the `detect` method in `Strategy` impl:
```rust
fn detect(&self, ctx: &DetectionContext) -> Vec<Opportunity> {
    detect_single_condition(ctx.pair, ctx.cache, &self.config, ctx.payout())
        .into_iter()
        .collect()
}
```

**Step 4: Run all tests to verify**

Run: `cargo test -p edgelord --lib single_condition`
Expected: All PASS

**Step 5: Commit**

```bash
git add src/core/strategy/single_condition.rs
git commit -m "Use payout from context in SingleConditionStrategy"
```

---

### Task 8: Update MarketRebalancingStrategy to Use Payout

**Files:**
- Modify: `src/core/strategy/market_rebalancing.rs`
- Test: Existing tests + new test

**Context:** Same change as Task 7 but for the market rebalancing strategy.

**Step 1: Read current file**

Read `src/core/strategy/market_rebalancing.rs` to understand current structure.

**Step 2: Write the failing test**

Add test to `src/core/strategy/market_rebalancing.rs`:

```rust
#[test]
fn test_rebalancing_with_custom_payout() {
    // Similar pattern to single_condition test
    // Test that $10 payout is respected in edge calculation
}
```

**Step 3: Implement the changes**

Update all `Decimal::ONE` references in `market_rebalancing.rs` to use payout from context.

**Step 4: Run all tests to verify**

Run: `cargo test -p edgelord --lib market_rebalancing`
Expected: All PASS

**Step 5: Commit**

```bash
git add src/core/strategy/market_rebalancing.rs
git commit -m "Use payout from context in MarketRebalancingStrategy"
```

---

### Task 9: Update OpportunityBuilder to Accept Payout

**Files:**
- Modify: `src/core/domain/opportunity.rs`
- Test: Existing tests + new test

**Context:** The `OpportunityBuilder` currently calculates edge as `Decimal::ONE - total_cost`. It needs to accept a payout parameter.

**Step 1: Write the failing test**

Add to `src/core/domain/opportunity.rs`:

```rust
#[test]
fn builder_with_custom_payout() {
    let opp = Opportunity::builder()
        .market_id(make_market_id())
        .question("Q?")
        .yes_token(make_yes_token(), dec!(4.00))
        .no_token(make_no_token(), dec!(5.00))
        .volume(dec!(10))
        .payout(dec!(10.00))
        .build()
        .unwrap();

    assert_eq!(opp.total_cost(), dec!(9.00));
    assert_eq!(opp.edge(), dec!(1.00)); // 10 - 9
    assert_eq!(opp.expected_profit(), dec!(10.00)); // 1 * 10
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p edgelord --lib opportunity::tests::builder_with_custom_payout`
Expected: FAIL - method not found

**Step 3: Implement the changes**

Add `payout` field to `OpportunityBuilder`:
```rust
payout: Option<Decimal>,
```

Add builder method:
```rust
/// Set the payout amount (defaults to 1.00).
#[must_use]
pub fn payout(mut self, payout: Decimal) -> Self {
    self.payout = Some(payout);
    self
}
```

Update `build` to use payout:
```rust
let payout = self.payout.unwrap_or(Decimal::ONE);
let edge = payout - total_cost;
```

**Step 4: Run all tests to verify**

Run: `cargo test -p edgelord --lib opportunity`
Expected: All PASS

**Step 5: Commit**

```bash
git add src/core/domain/opportunity.rs
git commit -m "Add payout parameter to OpportunityBuilder"
```

---

### Task 10: Update Orchestrator to Pass Payout to Strategies

**Files:**
- Modify: `src/app/orchestrator.rs`
- Test: Manual verification (integration)

**Context:** The orchestrator creates `DetectionContext` for strategy detection. It needs to pass the exchange's payout amount.

**Step 1: Read relevant parts of orchestrator**

Read `src/app/orchestrator.rs` to find where `DetectionContext` is created.

**Step 2: Implement the changes**

Update `DetectionContext::new` calls to `DetectionContext::with_payout` using Polymarket's payout constant:

```rust
use crate::core::exchange::polymarket::POLYMARKET_PAYOUT;

// In detection code:
let ctx = DetectionContext::with_payout(&pair, &cache, POLYMARKET_PAYOUT);
```

**Step 3: Verify compilation**

Run: `cargo build`
Expected: SUCCESS

**Step 4: Run full test suite**

Run: `cargo test`
Expected: All PASS

**Step 5: Commit**

```bash
git add src/app/orchestrator.rs
git commit -m "Pass exchange payout to detection context in orchestrator"
```

---

### Task 11: Add Backward-Compatible MarketPair from Market

**Files:**
- Modify: `src/core/domain/generic_market.rs`
- Test: Tests in same file

**Context:** For gradual migration, we need to convert between `Market` and legacy `MarketPair` for binary markets.

**Step 1: Write the failing test**

Add to `src/core/domain/generic_market.rs`:

```rust
#[test]
fn test_market_to_pair_binary() {
    let market = Market::new(
        MarketId::from("m1"),
        "Q?",
        vec![
            Outcome::new(TokenId::from("yes"), "Yes"),
            Outcome::new(TokenId::from("no"), "No"),
        ],
        dec!(1.00),
    );

    let pair = market.to_market_pair().unwrap();
    assert_eq!(pair.yes_token().as_str(), "yes");
    assert_eq!(pair.no_token().as_str(), "no");
}

#[test]
fn test_market_to_pair_non_binary_fails() {
    let market = Market::new(
        MarketId::from("m1"),
        "Q?",
        vec![
            Outcome::new(TokenId::from("a"), "A"),
            Outcome::new(TokenId::from("b"), "B"),
            Outcome::new(TokenId::from("c"), "C"),
        ],
        dec!(1.00),
    );

    assert!(market.to_market_pair().is_none());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p edgelord --lib generic_market::tests::test_market_to_pair`
Expected: FAIL - method not found

**Step 3: Implement the changes**

Add to `impl Market`:

```rust
/// Convert to legacy MarketPair if this is a binary market with Yes/No outcomes.
///
/// Returns None for non-binary markets or if Yes/No outcomes aren't found.
#[must_use]
pub fn to_market_pair(&self) -> Option<MarketPair> {
    if !self.is_binary() {
        return None;
    }

    let yes = self.outcome_by_name("Yes")?;
    let no = self.outcome_by_name("No")?;

    Some(MarketPair::new(
        self.market_id.clone(),
        &self.question,
        yes.token_id.clone(),
        no.token_id.clone(),
    ))
}
```

Add import at top:
```rust
use super::market::MarketPair;
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p edgelord --lib generic_market`
Expected: All PASS

**Step 5: Commit**

```bash
git add src/core/domain/generic_market.rs
git commit -m "Add to_market_pair conversion for backward compatibility"
```

---

### Task 12: Update Registry Factory to Use Generic Types

**Files:**
- Modify: `src/core/exchange/polymarket/registry.rs`
- Test: Existing tests

**Context:** The Polymarket registry should produce `Market` instances that can be converted to `MarketPair` for backward compatibility.

**Step 1: Implement factory method**

Add to `src/core/exchange/polymarket/registry.rs`:

```rust
use crate::core::domain::{Market, Outcome};
use crate::core::exchange::polymarket::POLYMARKET_PAYOUT;

impl MarketRegistry {
    /// Build a registry with generic Market types.
    ///
    /// This method creates both legacy MarketPair and new Market instances.
    pub fn from_markets_generic(markets: &[ApiMarket]) -> (Self, Vec<Market>) {
        let mut registry = Self::new();
        let mut generic_markets = Vec::new();

        for market in markets {
            // Create outcomes from all tokens
            let outcomes: Vec<Outcome> = market
                .tokens
                .iter()
                .map(|t| Outcome::new(
                    TokenId::from(t.token_id.clone()),
                    t.outcome.clone(),
                ))
                .collect();

            let generic = Market::new(
                MarketId::from(market.condition_id.clone()),
                market.question.clone().unwrap_or_default(),
                outcomes,
                POLYMARKET_PAYOUT,
            );

            // Add to legacy registry if binary
            if let Some(pair) = generic.to_market_pair() {
                registry.token_to_market.insert(pair.yes_token().clone(), pair.clone());
                registry.token_to_market.insert(pair.no_token().clone(), pair.clone());
                registry.pairs.push(pair);
            }

            generic_markets.push(generic);
        }

        (registry, generic_markets)
    }
}
```

**Step 2: Run tests**

Run: `cargo test -p edgelord --lib polymarket::registry`
Expected: All PASS

**Step 3: Commit**

```bash
git add src/core/exchange/polymarket/registry.rs
git commit -m "Add generic market factory to Polymarket registry"
```

---

### Task 13: Final Integration Test

**Files:**
- Create: `tests/multi_exchange_abstraction.rs`

**Context:** Integration test verifying the full abstraction works end-to-end.

**Step 1: Write integration test**

Create `tests/multi_exchange_abstraction.rs`:

```rust
//! Integration tests for multi-exchange abstraction.

use edgelord::core::domain::{
    GenericMarketRegistry, GenericOpportunity, Market, MarketId, OpportunityLeg, Outcome, TokenId,
};
use edgelord::core::exchange::{ExchangeConfig, polymarket::PolymarketExchangeConfig};
use rust_decimal_macros::dec;

#[test]
fn test_polymarket_config_integration() {
    let config = PolymarketExchangeConfig;

    // Verify Polymarket uses expected values
    assert_eq!(config.name(), "polymarket");
    assert_eq!(config.default_payout(), dec!(1.00));
    assert!(config.is_positive_outcome("Yes"));
    assert!(config.is_negative_outcome("No"));
}

#[test]
fn test_generic_market_registry_integration() {
    let mut registry = GenericMarketRegistry::new();

    // Add binary market
    registry.add(Market::new(
        MarketId::from("m1"),
        "Will it rain?",
        vec![
            Outcome::new(TokenId::from("yes-1"), "Yes"),
            Outcome::new(TokenId::from("no-1"), "No"),
        ],
        dec!(1.00),
    ));

    // Add multi-outcome market
    registry.add(Market::new(
        MarketId::from("m2"),
        "Who wins?",
        vec![
            Outcome::new(TokenId::from("trump"), "Trump"),
            Outcome::new(TokenId::from("biden"), "Biden"),
            Outcome::new(TokenId::from("other"), "Other"),
        ],
        dec!(1.00),
    ));

    assert_eq!(registry.len(), 2);
    assert_eq!(registry.binary_markets().count(), 1);
    assert_eq!(registry.multi_outcome_markets().count(), 1);
}

#[test]
fn test_generic_opportunity_with_payout() {
    // Simulate opportunity detection with custom payout
    let legs = vec![
        OpportunityLeg::new(TokenId::from("yes"), dec!(0.45)),
        OpportunityLeg::new(TokenId::from("no"), dec!(0.45)),
    ];

    let opp = GenericOpportunity::new(
        MarketId::from("m1"),
        "Test?",
        legs,
        dec!(100),
        dec!(1.00), // Polymarket payout
    );

    assert_eq!(opp.total_cost(), dec!(0.90));
    assert_eq!(opp.edge(), dec!(0.10));
    assert_eq!(opp.expected_profit(), dec!(10.00));
}

#[test]
fn test_backward_compatibility_market_pair() {
    let market = Market::new(
        MarketId::from("m1"),
        "Q?",
        vec![
            Outcome::new(TokenId::from("yes"), "Yes"),
            Outcome::new(TokenId::from("no"), "No"),
        ],
        dec!(1.00),
    );

    // Can convert to legacy MarketPair
    let pair = market.to_market_pair().expect("should convert binary market");
    assert_eq!(pair.yes_token().as_str(), "yes");
    assert_eq!(pair.no_token().as_str(), "no");
}
```

**Step 2: Run integration tests**

Run: `cargo test --test multi_exchange_abstraction`
Expected: All PASS

**Step 3: Commit**

```bash
git add tests/multi_exchange_abstraction.rs
git commit -m "Add integration tests for multi-exchange abstraction"
```

---

### Task 14: Documentation Update

**Files:**
- Modify: `src/core/domain/mod.rs` (module doc)
- Modify: `src/core/exchange/mod.rs` (module doc)

**Context:** Update module-level documentation to explain the multi-exchange architecture.

**Step 1: Update domain module docs**

Update `src/core/domain/mod.rs` module doc:

```rust
//! Core domain types for edgelord.
//!
//! This module contains the fundamental types used throughout the application:
//!
//! ## Exchange-Agnostic Types
//!
//! - [`Market`] - Generic market with N outcomes and configurable payout
//! - [`Outcome`] - A single outcome in a market
//! - [`GenericOpportunity`] - Arbitrage opportunity supporting any outcome structure
//! - [`GenericMarketRegistry`] - Registry mapping tokens to markets
//!
//! ## Legacy Binary-Market Types (Polymarket-compatible)
//!
//! - [`MarketPair`] - YES/NO market pair (use `Market::to_market_pair()` to convert)
//! - [`Opportunity`] - Binary arbitrage opportunity
//!
//! ## Identifier Types
//!
//! - [`MarketId`] - Unique market identifier
//! - [`TokenId`] - Unique token/outcome identifier
```

**Step 2: Update exchange module docs**

Update `src/core/exchange/mod.rs` module doc:

```rust
//! Exchange abstraction layer.
//!
//! ## Architecture
//!
//! Each exchange implements these traits:
//!
//! - [`ExchangeConfig`] - Static configuration (payout, outcome naming)
//! - [`MarketFetcher`] - Fetch market listings
//! - [`MarketDataStream`] - Real-time price feeds
//! - [`OrderExecutor`] - Order execution
//!
//! ## Adding a New Exchange
//!
//! 1. Create module under `exchange/<name>/`
//! 2. Implement `ExchangeConfig` with exchange-specific values
//! 3. Implement the async traits for data and execution
//! 4. Add to `ExchangeFactory`
```

**Step 3: Commit**

```bash
git add src/core/domain/mod.rs src/core/exchange/mod.rs
git commit -m "Update documentation for multi-exchange architecture"
```

---

## Summary

This plan creates a complete abstraction layer that:

1. **Supports N-outcome markets** via `Market` and `GenericOpportunity`
2. **Configurable payout amounts** via `ExchangeConfig` trait
3. **Flexible outcome naming** via exchange-provided conventions
4. **Backward compatible** with existing `MarketPair` and `Opportunity` types
5. **Polymarket keeps working** as the reference implementation

After completing all tasks, adding a new exchange requires:
1. Implement `ExchangeConfig` with exchange values
2. Implement data fetching traits
3. Add to factory
4. No changes to core strategy logic
