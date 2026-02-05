# Comprehensive Repository Restructure

> Status: Historical
> Superseded by: N/A
> Summary:
> - Goal: Transform edgelord into a 10/10 production-quality Rust project with clean architecture, rich error handling, proper encapsulation, and comprehensive documentation.
> - Scope: Target Structure
> Planned Outcomes:
> - Target Structure
> - Task 1: Create lib.rs and Restructure Entry Points


> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Transform edgelord into a 10/10 production-quality Rust project with clean architecture, rich error handling, proper encapsulation, and comprehensive documentation.

**Architecture:** Domain-driven design with exchange abstraction layer. Domain types are exchange-agnostic; exchange implementations (Polymarket, future Kalshi) implement common traits.

**Tech Stack:** Rust 2021, tokio async runtime, thiserror for errors, trait-based abstractions.

---

## Target Structure

```
src/
├── lib.rs                      # Library root with public API
├── main.rs                     # Thin binary entry point
│
├── error.rs                    # Rich, structured error types
├── config.rs                   # Configuration loading
│
├── domain/                     # Exchange-agnostic core
│   ├── mod.rs                  # Public exports
│   ├── id.rs                  # TokenId, MarketId (newtypes)
│   ├── money.rs                # Price, Volume (type aliases + helpers)
│   ├── orderbook.rs            # PriceLevel, OrderBook, OrderBookCache
│   ├── opportunity.rs          # Opportunity type with builder
│   ├── position.rs             # Position, PositionLeg, PositionTracker
│   ├── market.rs               # MarketPair, MarketInfo
│   └── detector.rs             # DetectorConfig, detection logic
│
├── exchange/                   # Exchange abstraction layer
│   ├── mod.rs
│   └── traits.rs               # ExchangeClient, OrderExecutor traits
│
├── polymarket/                 # Polymarket implementation
│   ├── mod.rs
│   ├── client.rs               # PolymarketClient (implements traits)
│   ├── executor.rs             # PolymarketExecutor (moved from executor/)
│   ├── websocket.rs            # WebSocketHandler
│   ├── messages.rs             # WS types + conversion to domain
│   ├── types.rs                # API response types
│   └── registry.rs             # MarketRegistry
│
└── app.rs                      # Application orchestration

tests/
├── common/mod.rs               # Test utilities
└── integration/
    └── detection_test.rs       # End-to-end detection tests
```

---

## Task 1: Create lib.rs and Restructure Entry Points

**Files:**
- Create: `src/lib.rs`
- Modify: `src/main.rs`

**Step 1: Create lib.rs with module declarations and public API**

```rust
//! Edgelord - Polymarket arbitrage detection and execution.
//!
//! # Architecture
//!
//! - `domain` - Exchange-agnostic types and logic
//! - `exchange` - Trait definitions for exchange implementations
//! - `polymarket` - Polymarket-specific implementation
//!
//! # Example
//!
//! ```no_run
//! use edgelord::config::Config;
//! use edgelord::domain::{DetectorConfig, OrderBookCache};
//! use edgelord::polymarket::PolymarketClient;
//!
//! #[tokio::main]
//! async fn main() -> edgelord::error::Result<()> {
//!     let config = Config::load("config.toml")?;
//!     let client = PolymarketClient::new(&config.network.api_url);
//!     Ok(())
//! }
//! ```

pub mod app;
pub mod config;
pub mod domain;
pub mod error;
pub mod exchange;
pub mod polymarket;
```

**Step 2: Simplify main.rs to thin entry point**

```rust
use edgelord::app::App;
use edgelord::config::Config;
use edgelord::error::Result;
use tokio::signal;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    let config = match Config::load("config.toml") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load config: {e}");
            std::process::exit(1);
        }
    };

    config.init_logging();
    info!("edgelord starting");

    tokio::select! {
        result = App::run(config) => {
            if let Err(e) = result {
                error!(error = %e, "Fatal error");
                std::process::exit(1);
            }
        }
        _ = signal::ctrl_c() => {
            info!("Shutdown signal received");
        }
    }

    info!("edgelord stopped");
}
```

**Step 3: Run cargo check**

Expected: Will fail (missing modules) - that's fine, we'll create them.

**Step 4: Commit**

```bash
git add src/lib.rs src/main.rs
git commit -m "refactor: add lib.rs and simplify main entry point"
```

---

## Task 2: Create Exchange Traits Module

**Files:**
- Create: `src/exchange/mod.rs`
- Create: `src/exchange/traits.rs`

**Step 1: Create exchange/mod.rs**

```rust
//! Exchange abstraction layer.
//!
//! Defines traits that exchange implementations must fulfill,
//! enabling multi-exchange support with a common interface.

mod traits;

pub use traits::{ExchangeClient, OrderExecutor, ExecutionResult, OrderId};
```

**Step 2: Create exchange/traits.rs with trait definitions**

```rust
//! Core traits for exchange implementations.

use async_trait::async_trait;
use std::fmt;

use crate::domain::{MarketInfo, Opportunity, Position, TokenId};
use crate::error::Result;

/// Unique order identifier returned by an exchange.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrderId(pub String);

impl fmt::Display for OrderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Result of executing an arbitrage opportunity.
#[derive(Debug, Clone)]
pub enum ExecutionResult {
    /// Both legs executed successfully.
    Success {
        yes_order: OrderId,
        no_order: OrderId,
        position: Position,
    },
    /// One leg succeeded, one failed - exposure exists.
    PartialFill {
        filled_order: OrderId,
        filled_leg: TokenId,
        failed_leg: TokenId,
        error: String,
    },
    /// Both legs failed.
    Failed {
        reason: String,
    },
}

impl ExecutionResult {
    /// Returns true if execution was fully successful.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// Returns true if a partial fill occurred (exposure exists).
    pub fn is_partial(&self) -> bool {
        matches!(self, Self::PartialFill { .. })
    }
}

/// Client for fetching market data from an exchange.
#[async_trait]
pub trait ExchangeClient: Send + Sync {
    /// Fetch available markets from the exchange.
    async fn get_markets(&self, limit: usize) -> Result<Vec<MarketInfo>>;
}

/// Executor for submitting orders to an exchange.
#[async_trait]
pub trait OrderExecutor: Send + Sync {
    /// Execute an arbitrage opportunity by placing orders on both legs.
    async fn execute(&self, opportunity: &Opportunity) -> Result<ExecutionResult>;
}
```

**Step 3: Add async-trait to Cargo.toml**

```toml
# Async traits
async-trait = "0.1"
```

**Step 4: Run cargo check**

Expected: Compiles (with some missing type errors we'll fix).

**Step 5: Commit**

```bash
git add src/exchange/ Cargo.toml
git commit -m "feat: add exchange abstraction traits"
```

---

## Task 3: Restructure Domain Module - Core Types

**Files:**
- Create: `src/domain/id.rs`
- Create: `src/domain/money.rs`
- Create: `src/domain/market.rs`
- Modify: `src/domain/mod.rs`
- Delete content from: `src/domain/types.rs` (will be removed)

**Step 1: Create domain/id.rs with identifier newtypes**

```rust
//! Strongly-typed identifiers.

use std::fmt;

/// Token identifier - unique across all exchanges.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TokenId(String);

impl TokenId {
    /// Create a new token ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the underlying string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TokenId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for TokenId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for TokenId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// Market/condition identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MarketId(String);

impl MarketId {
    /// Create a new market ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the underlying string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MarketId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for MarketId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}
```

**Step 2: Create domain/money.rs with price/volume types**

```rust
//! Monetary types - always use Decimal, never floats.

use rust_decimal::Decimal;

/// Price in dollars (0.00 to 1.00 for prediction markets).
pub type Price = Decimal;

/// Volume/quantity of shares.
pub type Volume = Decimal;

/// Useful constants for price calculations.
pub mod constants {
    use rust_decimal::Decimal;

    /// One dollar - the payout for a winning prediction.
    pub const ONE_DOLLAR: Decimal = Decimal::ONE;

    /// Zero dollars.
    pub const ZERO: Decimal = Decimal::ZERO;
}
```

**Step 3: Create domain/market.rs with market types**

```rust
//! Market-related types.

use super::{MarketId, TokenId};

/// Information about a tradeable market.
#[derive(Debug, Clone)]
pub struct MarketInfo {
    /// Unique market identifier.
    pub id: MarketId,
    /// Human-readable question/description.
    pub question: String,
    /// Available tokens/outcomes.
    pub tokens: Vec<TokenInfo>,
}

/// Information about a token/outcome within a market.
#[derive(Debug, Clone)]
pub struct TokenInfo {
    /// Unique token identifier.
    pub id: TokenId,
    /// Outcome name (e.g., "Yes", "No", "Trump", "Harris").
    pub outcome: String,
}

/// A YES/NO binary market pair.
#[derive(Debug, Clone)]
pub struct MarketPair {
    market_id: MarketId,
    question: String,
    yes_token: TokenId,
    no_token: TokenId,
}

impl MarketPair {
    /// Create a new market pair.
    pub fn new(
        market_id: MarketId,
        question: impl Into<String>,
        yes_token: TokenId,
        no_token: TokenId,
    ) -> Self {
        Self {
            market_id,
            question: question.into(),
            yes_token,
            no_token,
        }
    }

    pub fn market_id(&self) -> &MarketId { &self.market_id }
    pub fn question(&self) -> &str { &self.question }
    pub fn yes_token(&self) -> &TokenId { &self.yes_token }
    pub fn no_token(&self) -> &TokenId { &self.no_token }
}
```

**Step 4: Update domain/mod.rs with new structure**

```rust
//! Exchange-agnostic domain logic.
//!
//! This module contains core types and business logic that work
//! regardless of which exchange is being used.

mod detector;
mod ids;
mod market;
mod money;
mod opportunity;
mod orderbook;
mod position;

// Core identifiers
pub use ids::{MarketId, TokenId};

// Money types
pub use money::{Price, Volume, constants};

// Market types
pub use market::{MarketInfo, MarketPair, TokenInfo};

// Order book
pub use orderbook::{OrderBook, OrderBookCache, PriceLevel};

// Opportunities
pub use opportunity::Opportunity;

// Positions
pub use position::{Position, PositionId, PositionLeg, PositionStatus, PositionTracker};

// Detection
pub use detector::{detect_single_condition, DetectorConfig};
```

**Step 5: Run cargo check, fix any import issues**

**Step 6: Commit**

```bash
git add src/domain/
git commit -m "refactor: restructure domain module with proper encapsulation"
```

---

## Task 4: Restructure OrderBook with Clean Separation

**Files:**
- Modify: `src/domain/orderbook.rs`
- Modify: `src/polymarket/messages.rs`

**Step 1: Rewrite domain/orderbook.rs without polymarket imports**

```rust
//! Order book types and caching.

use parking_lot::RwLock;
use std::collections::HashMap;

use super::{Price, TokenId, Volume};

/// A single price level in an order book.
#[derive(Debug, Clone)]
pub struct PriceLevel {
    price: Price,
    size: Volume,
}

impl PriceLevel {
    /// Create a new price level.
    pub fn new(price: Price, size: Volume) -> Self {
        Self { price, size }
    }

    pub fn price(&self) -> Price { self.price }
    pub fn size(&self) -> Volume { self.size }
}

/// Order book for a single token.
#[derive(Debug, Clone)]
pub struct OrderBook {
    token_id: TokenId,
    bids: Vec<PriceLevel>,  // Sorted descending by price
    asks: Vec<PriceLevel>,  // Sorted ascending by price
}

impl OrderBook {
    /// Create a new empty order book.
    pub fn new(token_id: TokenId) -> Self {
        Self {
            token_id,
            bids: Vec::new(),
            asks: Vec::new(),
        }
    }

    /// Create an order book with existing levels.
    pub fn with_levels(token_id: TokenId, bids: Vec<PriceLevel>, asks: Vec<PriceLevel>) -> Self {
        Self { token_id, bids, asks }
    }

    pub fn token_id(&self) -> &TokenId { &self.token_id }
    pub fn bids(&self) -> &[PriceLevel] { &self.bids }
    pub fn asks(&self) -> &[PriceLevel] { &self.asks }

    /// Best bid (highest buy price).
    pub fn best_bid(&self) -> Option<&PriceLevel> {
        self.bids.first()
    }

    /// Best ask (lowest sell price).
    pub fn best_ask(&self) -> Option<&PriceLevel> {
        self.asks.first()
    }
}

/// Thread-safe cache of order books.
#[derive(Debug)]
pub struct OrderBookCache {
    books: RwLock<HashMap<TokenId, OrderBook>>,
}

impl OrderBookCache {
    /// Create a new empty cache.
    pub fn new() -> Self {
        Self {
            books: RwLock::new(HashMap::new()),
        }
    }

    /// Insert or update an order book.
    pub fn update(&self, book: OrderBook) {
        self.books.write().insert(book.token_id().clone(), book);
    }

    /// Get a snapshot of an order book.
    pub fn get(&self, token_id: &TokenId) -> Option<OrderBook> {
        self.books.read().get(token_id).cloned()
    }

    /// Get snapshots of two order books atomically.
    pub fn get_pair(&self, token_a: &TokenId, token_b: &TokenId) -> (Option<OrderBook>, Option<OrderBook>) {
        let books = self.books.read();
        (books.get(token_a).cloned(), books.get(token_b).cloned())
    }

    /// Number of books in cache.
    pub fn len(&self) -> usize {
        self.books.read().len()
    }

    /// Check if cache is empty.
    pub fn is_empty(&self) -> bool {
        self.books.read().is_empty()
    }
}

impl Default for OrderBookCache {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 2: Update polymarket/messages.rs to convert to domain types**

Add conversion method to BookMessage:

```rust
use crate::domain::{OrderBook, PriceLevel, TokenId};

impl BookMessage {
    /// Convert to domain OrderBook type.
    pub fn to_orderbook(&self) -> OrderBook {
        let token_id = TokenId::from(self.asset_id.clone());

        let bids = self.bids.iter()
            .filter_map(|pl| {
                let price = pl.price.parse().ok()?;
                let size = pl.size.parse().ok()?;
                Some(PriceLevel::new(price, size))
            })
            .collect();

        let asks = self.asks.iter()
            .filter_map(|pl| {
                let price = pl.price.parse().ok()?;
                let size = pl.size.parse().ok()?;
                Some(PriceLevel::new(price, size))
            })
            .collect();

        OrderBook::with_levels(token_id, bids, asks)
    }
}
```

**Step 3: Update callers to use new conversion**

In main.rs/app.rs, change:
```rust
// Old
cache.update_from_ws(&book);

// New
cache.update(book.to_orderbook());
```

**Step 4: Run cargo check and tests**

**Step 5: Commit**

```bash
git add src/domain/orderbook.rs src/polymarket/messages.rs
git commit -m "refactor: clean separation between domain and polymarket types"
```

---

## Task 5: Create Opportunity Module with Builder

**Files:**
- Create: `src/domain/opportunity.rs`

**Step 1: Create opportunity.rs with builder pattern**

```rust
//! Arbitrage opportunity types.

use super::{MarketId, Price, TokenId, Volume};

/// A detected arbitrage opportunity.
#[derive(Debug, Clone)]
pub struct Opportunity {
    market_id: MarketId,
    question: String,
    yes_token: TokenId,
    no_token: TokenId,
    yes_ask: Price,
    no_ask: Price,
    total_cost: Price,
    edge: Price,
    volume: Volume,
    expected_profit: Price,
}

impl Opportunity {
    /// Create a builder for constructing an opportunity.
    pub fn builder() -> OpportunityBuilder {
        OpportunityBuilder::default()
    }

    // Accessors
    pub fn market_id(&self) -> &MarketId { &self.market_id }
    pub fn question(&self) -> &str { &self.question }
    pub fn yes_token(&self) -> &TokenId { &self.yes_token }
    pub fn no_token(&self) -> &TokenId { &self.no_token }
    pub fn yes_ask(&self) -> Price { self.yes_ask }
    pub fn no_ask(&self) -> Price { self.no_ask }
    pub fn total_cost(&self) -> Price { self.total_cost }
    pub fn edge(&self) -> Price { self.edge }
    pub fn volume(&self) -> Volume { self.volume }
    pub fn expected_profit(&self) -> Price { self.expected_profit }
}

/// Builder for creating opportunities with validation.
#[derive(Default)]
pub struct OpportunityBuilder {
    market_id: Option<MarketId>,
    question: Option<String>,
    yes_token: Option<TokenId>,
    no_token: Option<TokenId>,
    yes_ask: Option<Price>,
    no_ask: Option<Price>,
    volume: Option<Volume>,
}

impl OpportunityBuilder {
    pub fn market_id(mut self, id: MarketId) -> Self {
        self.market_id = Some(id);
        self
    }

    pub fn question(mut self, q: impl Into<String>) -> Self {
        self.question = Some(q.into());
        self
    }

    pub fn yes_token(mut self, token: TokenId, ask_price: Price) -> Self {
        self.yes_token = Some(token);
        self.yes_ask = Some(ask_price);
        self
    }

    pub fn no_token(mut self, token: TokenId, ask_price: Price) -> Self {
        self.no_token = Some(token);
        self.no_ask = Some(ask_price);
        self
    }

    pub fn volume(mut self, v: Volume) -> Self {
        self.volume = Some(v);
        self
    }

    /// Build the opportunity, calculating derived fields.
    pub fn build(self) -> Result<Opportunity, OpportunityBuildError> {
        let market_id = self.market_id.ok_or(OpportunityBuildError::MissingField("market_id"))?;
        let question = self.question.ok_or(OpportunityBuildError::MissingField("question"))?;
        let yes_token = self.yes_token.ok_or(OpportunityBuildError::MissingField("yes_token"))?;
        let no_token = self.no_token.ok_or(OpportunityBuildError::MissingField("no_token"))?;
        let yes_ask = self.yes_ask.ok_or(OpportunityBuildError::MissingField("yes_ask"))?;
        let no_ask = self.no_ask.ok_or(OpportunityBuildError::MissingField("no_ask"))?;
        let volume = self.volume.ok_or(OpportunityBuildError::MissingField("volume"))?;

        let total_cost = yes_ask + no_ask;
        let edge = Price::ONE - total_cost;
        let expected_profit = edge * volume;

        Ok(Opportunity {
            market_id,
            question,
            yes_token,
            no_token,
            yes_ask,
            no_ask,
            total_cost,
            edge,
            volume,
            expected_profit,
        })
    }
}

/// Error building an opportunity.
#[derive(Debug, Clone)]
pub enum OpportunityBuildError {
    MissingField(&'static str),
}

impl std::fmt::Display for OpportunityBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingField(field) => write!(f, "missing required field: {}", field),
        }
    }
}

impl std::error::Error for OpportunityBuildError {}
```

**Step 2: Update detector.rs to use builder**

**Step 3: Run cargo check and tests**

**Step 4: Commit**

```bash
git add src/domain/opportunity.rs src/domain/detector.rs
git commit -m "feat: add Opportunity builder with validation"
```

---

## Task 6: Move Position Tracking to Domain

**Files:**
- Create: `src/domain/position.rs`
- Delete: `src/executor/positions.rs`
- Modify: `src/executor/mod.rs` (or delete entirely)

**Step 1: Create domain/position.rs (moved and improved)**

```rust
//! Position tracking for arbitrage trades.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use super::{MarketId, Price, TokenId, Volume};

/// Unique position identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PositionId(u64);

impl PositionId {
    /// Create a new position ID.
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the underlying value.
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for PositionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Status of a position.
#[derive(Debug, Clone)]
pub enum PositionStatus {
    /// All legs filled successfully.
    Open,
    /// Some legs filled, exposure exists.
    PartialFill {
        filled: Vec<TokenId>,
        missing: Vec<TokenId>,
    },
    /// Position closed (market settled or sold).
    Closed {
        pnl: Price,
        closed_at: DateTime<Utc>,
    },
}

impl PositionStatus {
    /// Check if position is open.
    pub fn is_open(&self) -> bool {
        matches!(self, Self::Open)
    }

    /// Check if position has partial fill (exposure).
    pub fn is_partial(&self) -> bool {
        matches!(self, Self::PartialFill { .. })
    }

    /// Check if position is closed.
    pub fn is_closed(&self) -> bool {
        matches!(self, Self::Closed { .. })
    }
}

/// A single leg of a position.
#[derive(Debug, Clone)]
pub struct PositionLeg {
    token_id: TokenId,
    size: Volume,
    entry_price: Price,
}

impl PositionLeg {
    /// Create a new position leg.
    pub fn new(token_id: TokenId, size: Volume, entry_price: Price) -> Self {
        Self { token_id, size, entry_price }
    }

    pub fn token_id(&self) -> &TokenId { &self.token_id }
    pub fn size(&self) -> Volume { self.size }
    pub fn entry_price(&self) -> Price { self.entry_price }

    /// Cost of this leg (size * price).
    pub fn cost(&self) -> Price {
        self.size * self.entry_price
    }
}

/// An arbitrage position (YES + NO tokens held).
#[derive(Debug, Clone)]
pub struct Position {
    id: PositionId,
    market_id: MarketId,
    legs: Vec<PositionLeg>,
    entry_cost: Price,
    guaranteed_payout: Price,
    opened_at: DateTime<Utc>,
    status: PositionStatus,
}

impl Position {
    /// Create a new open position.
    pub fn new(
        id: PositionId,
        market_id: MarketId,
        legs: Vec<PositionLeg>,
        guaranteed_payout: Price,
    ) -> Self {
        let entry_cost = legs.iter().map(|l| l.cost()).sum();
        Self {
            id,
            market_id,
            legs,
            entry_cost,
            guaranteed_payout,
            opened_at: Utc::now(),
            status: PositionStatus::Open,
        }
    }

    // Accessors
    pub fn id(&self) -> PositionId { self.id }
    pub fn market_id(&self) -> &MarketId { &self.market_id }
    pub fn legs(&self) -> &[PositionLeg] { &self.legs }
    pub fn entry_cost(&self) -> Price { self.entry_cost }
    pub fn guaranteed_payout(&self) -> Price { self.guaranteed_payout }
    pub fn opened_at(&self) -> DateTime<Utc> { self.opened_at }
    pub fn status(&self) -> &PositionStatus { &self.status }

    /// Expected profit (guaranteed payout - entry cost).
    pub fn expected_profit(&self) -> Price {
        self.guaranteed_payout - self.entry_cost
    }

    /// Check if position is still open.
    pub fn is_open(&self) -> bool {
        self.status.is_open()
    }

    /// Close the position with realized P&L.
    pub fn close(&mut self, pnl: Price) {
        self.status = PositionStatus::Closed {
            pnl,
            closed_at: Utc::now(),
        };
    }
}

/// Tracks all positions.
#[derive(Debug)]
pub struct PositionTracker {
    positions: Vec<Position>,
    next_id: u64,
}

impl PositionTracker {
    /// Create a new position tracker.
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            next_id: 1,
        }
    }

    /// Generate the next position ID.
    pub fn next_id(&mut self) -> PositionId {
        let id = PositionId::new(self.next_id);
        self.next_id += 1;
        id
    }

    /// Add a position to tracking.
    pub fn add(&mut self, position: Position) {
        self.positions.push(position);
    }

    /// Get all open positions.
    pub fn open_positions(&self) -> impl Iterator<Item = &Position> {
        self.positions.iter().filter(|p| p.is_open())
    }

    /// Total exposure (sum of entry costs for open positions).
    pub fn total_exposure(&self) -> Price {
        self.open_positions()
            .map(|p| p.entry_cost())
            .fold(Decimal::ZERO, |acc, cost| acc + cost)
    }

    /// Count of open positions.
    pub fn open_count(&self) -> usize {
        self.open_positions().count()
    }

    /// Get a position by ID.
    pub fn get(&self, id: PositionId) -> Option<&Position> {
        self.positions.iter().find(|p| p.id() == id)
    }

    /// Get a mutable position by ID.
    pub fn get_mut(&mut self, id: PositionId) -> Option<&mut Position> {
        self.positions.iter_mut().find(|p| p.id() == id)
    }
}

impl Default for PositionTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_position_tracker_empty() {
        let tracker = PositionTracker::new();
        assert_eq!(tracker.open_count(), 0);
        assert_eq!(tracker.total_exposure(), dec!(0));
    }

    #[test]
    fn test_position_id_increments() {
        let mut tracker = PositionTracker::new();
        assert_eq!(tracker.next_id().value(), 1);
        assert_eq!(tracker.next_id().value(), 2);
        assert_eq!(tracker.next_id().value(), 3);
    }

    #[test]
    fn test_position_entry_cost_calculated() {
        let legs = vec![
            PositionLeg::new(TokenId::new("yes"), dec!(100), dec!(0.45)),
            PositionLeg::new(TokenId::new("no"), dec!(100), dec!(0.50)),
        ];
        let position = Position::new(
            PositionId::new(1),
            MarketId::new("market"),
            legs,
            dec!(100),
        );

        assert_eq!(position.entry_cost(), dec!(95)); // 45 + 50
        assert_eq!(position.expected_profit(), dec!(5)); // 100 - 95
    }

    #[test]
    fn test_closed_positions_excluded_from_exposure() {
        let mut tracker = PositionTracker::new();

        let mut position = Position::new(
            tracker.next_id(),
            MarketId::new("market"),
            vec![PositionLeg::new(TokenId::new("t"), dec!(10), dec!(0.5))],
            dec!(10),
        );
        position.close(dec!(5));
        tracker.add(position);

        assert_eq!(tracker.open_count(), 0);
        assert_eq!(tracker.total_exposure(), dec!(0));
    }
}
```

**Step 2: Delete src/executor/ directory**

**Step 3: Run cargo check**

**Step 4: Commit**

```bash
git add src/domain/position.rs
git rm -r src/executor/
git commit -m "refactor: move position tracking to domain module"
```

---

## Task 7: Move Order Execution to Polymarket Module

**Files:**
- Create: `src/polymarket/executor.rs`
- Modify: `src/polymarket/mod.rs`

**Step 1: Create polymarket/executor.rs (adapted from old executor/orders.rs)**

```rust
//! Polymarket order execution.

use std::str::FromStr;
use std::sync::Arc;

use alloy_signer_local::PrivateKeySigner;
use parking_lot::Mutex;
use polymarket_client_sdk::auth::state::Authenticated;
use polymarket_client_sdk::auth::Normal;
use polymarket_client_sdk::clob::types::Side;
use polymarket_client_sdk::clob::{Client, Config as ClobConfig};
use polymarket_client_sdk::types::U256;
use rust_decimal::Decimal;
use tracing::{info, warn};

use crate::config::Config;
use crate::domain::{
    MarketId, Opportunity, Position, PositionId, PositionLeg, PositionTracker, Price, TokenId,
};
use crate::error::{Error, Result};
use crate::exchange::{ExecutionResult, OrderExecutor, OrderId};

/// Type alias for authenticated client.
type AuthenticatedClient = Client<Authenticated<Normal>>;

/// Polymarket order executor.
pub struct PolymarketExecutor {
    client: Arc<AuthenticatedClient>,
    signer: Arc<PrivateKeySigner>,
    positions: Mutex<PositionTracker>,
}

impl PolymarketExecutor {
    /// Create and authenticate a new executor.
    pub async fn new(config: &Config) -> Result<Self> {
        let private_key = config
            .wallet
            .private_key
            .as_ref()
            .ok_or_else(|| Error::Config("WALLET_PRIVATE_KEY not set".into()))?;

        let signer = PrivateKeySigner::from_str(private_key)
            .map_err(|e| Error::Config(format!("Invalid private key: {e}")))?
            .with_chain_id(Some(config.network.chain_id));

        info!(
            chain_id = config.network.chain_id,
            address = %signer.address(),
            "Authenticating with Polymarket CLOB"
        );

        let client = Client::new(&config.network.api_url, ClobConfig::default())
            .map_err(|e| Error::Execution(format!("Failed to create client: {e}")))?
            .authentication_builder(&signer)
            .authenticate()
            .await
            .map_err(|e| Error::Execution(format!("Authentication failed: {e}")))?;

        info!("CLOB client authenticated successfully");

        Ok(Self {
            client: Arc::new(client),
            signer: Arc::new(signer),
            positions: Mutex::new(PositionTracker::new()),
        })
    }

    /// Get current total exposure.
    pub fn total_exposure(&self) -> Price {
        self.positions.lock().total_exposure()
    }

    /// Get count of open positions.
    pub fn open_position_count(&self) -> usize {
        self.positions.lock().open_count()
    }

    /// Submit a single order.
    async fn submit_order(
        &self,
        token_id: &str,
        side: Side,
        size: Decimal,
        price: Decimal,
    ) -> Result<OrderId> {
        let token_u256 = U256::from_str(token_id)
            .map_err(|e| Error::Execution(format!("Invalid token ID: {e}")))?;

        let order = self.client
            .limit_order()
            .token_id(token_u256)
            .side(side)
            .price(price)
            .size(size)
            .build()
            .await
            .map_err(|e| Error::Execution(format!("Failed to build order: {e}")))?;

        let signed = self.client
            .sign(self.signer.as_ref(), order)
            .await
            .map_err(|e| Error::Execution(format!("Failed to sign order: {e}")))?;

        let response = self.client
            .post_order(signed)
            .await
            .map_err(|e| Error::Execution(format!("Failed to submit order: {e}")))?;

        info!(
            order_id = %response.order_id,
            token = token_id,
            side = ?side,
            size = %size,
            price = %price,
            "Order submitted"
        );

        Ok(OrderId(response.order_id))
    }

    /// Record a successful position.
    fn record_position(&self, opportunity: &Opportunity) -> Position {
        let mut tracker = self.positions.lock();
        let id = tracker.next_id();

        let legs = vec![
            PositionLeg::new(
                opportunity.yes_token().clone(),
                opportunity.volume(),
                opportunity.yes_ask(),
            ),
            PositionLeg::new(
                opportunity.no_token().clone(),
                opportunity.volume(),
                opportunity.no_ask(),
            ),
        ];

        let position = Position::new(
            id,
            opportunity.market_id().clone(),
            legs,
            opportunity.volume(), // Guaranteed $1 per share pair
        );

        info!(
            position_id = %position.id(),
            entry_cost = %position.entry_cost(),
            expected_profit = %position.expected_profit(),
            "Position opened"
        );

        tracker.add(position.clone());
        position
    }
}

#[async_trait::async_trait]
impl OrderExecutor for PolymarketExecutor {
    async fn execute(&self, opportunity: &Opportunity) -> Result<ExecutionResult> {
        info!(
            market = %opportunity.market_id(),
            edge = %opportunity.edge(),
            volume = %opportunity.volume(),
            "Executing arbitrage"
        );

        let (yes_result, no_result) = tokio::join!(
            self.submit_order(
                opportunity.yes_token().as_str(),
                Side::Buy,
                opportunity.volume(),
                opportunity.yes_ask(),
            ),
            self.submit_order(
                opportunity.no_token().as_str(),
                Side::Buy,
                opportunity.volume(),
                opportunity.no_ask(),
            ),
        );

        match (yes_result, no_result) {
            (Ok(yes_order), Ok(no_order)) => {
                let position = self.record_position(opportunity);
                info!(
                    yes_order = %yes_order,
                    no_order = %no_order,
                    "Both legs executed successfully"
                );
                Ok(ExecutionResult::Success {
                    yes_order,
                    no_order,
                    position,
                })
            }
            (Ok(yes_order), Err(no_err)) => {
                warn!(yes_order = %yes_order, error = %no_err, "NO leg failed");
                Ok(ExecutionResult::PartialFill {
                    filled_order: yes_order,
                    filled_leg: opportunity.yes_token().clone(),
                    failed_leg: opportunity.no_token().clone(),
                    error: no_err.to_string(),
                })
            }
            (Err(yes_err), Ok(no_order)) => {
                warn!(no_order = %no_order, error = %yes_err, "YES leg failed");
                Ok(ExecutionResult::PartialFill {
                    filled_order: no_order,
                    filled_leg: opportunity.no_token().clone(),
                    failed_leg: opportunity.yes_token().clone(),
                    error: yes_err.to_string(),
                })
            }
            (Err(yes_err), Err(no_err)) => {
                warn!(yes_error = %yes_err, no_error = %no_err, "Both legs failed");
                Ok(ExecutionResult::Failed {
                    reason: format!("YES: {yes_err}, NO: {no_err}"),
                })
            }
        }
    }
}
```

**Step 2: Update polymarket/mod.rs**

```rust
//! Polymarket exchange integration.

mod client;
mod executor;
mod messages;
mod registry;
mod types;
mod websocket;

pub use client::PolymarketClient;
pub use executor::PolymarketExecutor;
pub use messages::{BookMessage, WsMessage};
pub use registry::MarketRegistry;
pub use types::{Market, Token};
pub use websocket::WebSocketHandler;
```

**Step 3: Run cargo check**

**Step 4: Commit**

```bash
git add src/polymarket/executor.rs src/polymarket/mod.rs
git commit -m "refactor: move order execution to polymarket module"
```

---

## Task 8: Create App Orchestration Module

**Files:**
- Create: `src/app.rs`
- Modify: `src/main.rs`

**Step 1: Create app.rs with application logic**

```rust
//! Application orchestration.

use std::sync::Arc;

use tracing::{error, info, warn};

use crate::config::Config;
use crate::domain::{detect_single_condition, Opportunity, OrderBookCache, TokenId};
use crate::error::Result;
use crate::exchange::{ExecutionResult, OrderExecutor};
use crate::polymarket::{MarketRegistry, PolymarketClient, PolymarketExecutor, WebSocketHandler, WsMessage};

/// Application entry point.
pub struct App;

impl App {
    /// Run the application.
    pub async fn run(config: Config) -> Result<()> {
        // Initialize executor if wallet configured
        let executor = Self::init_executor(&config).await;

        // Fetch markets
        let client = PolymarketClient::new(&config.network.api_url);
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
            info!(
                market_id = %pair.market_id(),
                question = %pair.question(),
                "Tracking market"
            );
        }

        // Subscribe to orderbooks
        let token_ids: Vec<String> = registry
            .pairs()
            .iter()
            .flat_map(|p| vec![
                p.yes_token().as_str().to_string(),
                p.no_token().as_str().to_string(),
            ])
            .collect();

        info!(tokens = token_ids.len(), "Subscribing to orderbooks");

        // Run websocket handler
        let cache = Arc::new(OrderBookCache::new());
        let registry = Arc::new(registry);
        let detector_config = Arc::new(config.detector.clone());

        let handler = WebSocketHandler::new(config.network.ws_url);

        handler
            .run(token_ids, {
                let cache = cache.clone();
                let registry = registry.clone();
                let detector_config = detector_config.clone();
                let executor = executor.clone();

                move |msg| {
                    Self::handle_message(
                        msg,
                        &cache,
                        &registry,
                        &detector_config,
                        executor.clone(),
                    );
                }
            })
            .await?;

        Ok(())
    }

    async fn init_executor(config: &Config) -> Option<Arc<PolymarketExecutor>> {
        if config.wallet.private_key.is_none() {
            info!("No wallet configured - detection only mode");
            return None;
        }

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
    }

    fn handle_message(
        msg: WsMessage,
        cache: &OrderBookCache,
        registry: &MarketRegistry,
        config: &crate::domain::DetectorConfig,
        executor: Option<Arc<PolymarketExecutor>>,
    ) {
        if let WsMessage::Book(book) = msg {
            cache.update(book.to_orderbook());

            let token_id = TokenId::from(book.asset_id.clone());
            if let Some(pair) = registry.get_market_for_token(&token_id) {
                if let Some(opp) = detect_single_condition(pair, cache, config) {
                    Self::log_opportunity(&opp);

                    if let Some(exec) = executor {
                        Self::spawn_execution(exec, opp);
                    }
                }
            }
        }
    }

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

    fn spawn_execution(executor: Arc<PolymarketExecutor>, opportunity: Opportunity) {
        tokio::spawn(async move {
            match executor.execute(&opportunity).await {
                Ok(result) => match &result {
                    ExecutionResult::Success { .. } => {
                        info!("Execution successful");
                    }
                    ExecutionResult::PartialFill { error, .. } => {
                        warn!(error = %error, "Partial fill - exposure exists!");
                    }
                    ExecutionResult::Failed { reason } => {
                        warn!(reason = %reason, "Execution failed");
                    }
                },
                Err(e) => {
                    error!(error = %e, "Execution error");
                }
            }
        });
    }
}
```

**Step 2: Simplify main.rs to use App**

(Already shown in Task 1)

**Step 3: Run cargo check and tests**

**Step 4: Commit**

```bash
git add src/app.rs src/main.rs
git commit -m "refactor: extract app orchestration module"
```

---

## Task 9: Enrich Error Types

**Files:**
- Modify: `src/error.rs`

**Step 1: Replace string-based errors with structured types**

```rust
//! Error types for edgelord.

use thiserror::Error;

use crate::domain::TokenId;

/// Top-level error type.
#[derive(Error, Debug)]
pub enum Error {
    /// Configuration errors.
    #[error("configuration error: {0}")]
    Config(#[from] ConfigError),

    /// WebSocket errors.
    #[error("websocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    /// JSON parsing errors.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// HTTP errors.
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    /// IO errors.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// URL parsing errors.
    #[error("url error: {0}")]
    Url(#[from] url::ParseError),

    /// Execution errors.
    #[error("execution error: {0}")]
    Execution(#[from] ExecutionError),

    /// Polymarket SDK errors.
    #[error("polymarket error: {0}")]
    Polymarket(#[from] polymarket_client_sdk::error::Error),
}

/// Configuration-related errors.
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("missing required field: {field}")]
    MissingField { field: &'static str },

    #[error("invalid value for {field}: {message}")]
    InvalidValue { field: &'static str, message: String },

    #[error("failed to read config file: {0}")]
    ReadFile(#[source] std::io::Error),

    #[error("failed to parse config: {0}")]
    Parse(#[source] toml::de::Error),
}

/// Order execution errors.
#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("authentication failed: {reason}")]
    AuthFailed { reason: String },

    #[error("invalid token ID '{token_id}': {reason}")]
    InvalidTokenId { token_id: String, reason: String },

    #[error("order rejected: {reason}")]
    OrderRejected { reason: String },

    #[error("order build failed: {reason}")]
    OrderBuildFailed { reason: String },

    #[error("signing failed: {reason}")]
    SigningFailed { reason: String },

    #[error("submission failed: {reason}")]
    SubmissionFailed { reason: String },

    #[error("partial fill: {filled_leg} filled, {failed_leg} failed - {error}")]
    PartialFill {
        filled_leg: TokenId,
        failed_leg: TokenId,
        error: String,
    },
}

/// Result type alias.
pub type Result<T> = std::result::Result<T, Error>;
```

**Step 2: Update callers to use new error types**

**Step 3: Run cargo check and tests**

**Step 4: Commit**

```bash
git add src/error.rs
git commit -m "refactor: enrich error types with structured variants"
```

---

## Task 10: Update Detector to Use New Types

**Files:**
- Modify: `src/domain/detector.rs`

**Step 1: Update detector to use Opportunity builder and new accessors**

```rust
//! Arbitrage detection logic.

use rust_decimal::Decimal;
use serde::Deserialize;

use super::{MarketPair, Opportunity, OrderBookCache};

/// Configuration for the arbitrage detector.
#[derive(Debug, Clone, Deserialize)]
pub struct DetectorConfig {
    /// Minimum edge (profit per $1) to consider.
    #[serde(default = "default_min_edge")]
    pub min_edge: Decimal,

    /// Minimum expected profit to act on.
    #[serde(default = "default_min_profit")]
    pub min_profit: Decimal,
}

fn default_min_edge() -> Decimal {
    Decimal::new(5, 2) // 0.05
}

fn default_min_profit() -> Decimal {
    Decimal::new(50, 2) // 0.50
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            min_edge: default_min_edge(),
            min_profit: default_min_profit(),
        }
    }
}

/// Detect single-condition arbitrage (YES + NO < $1.00).
///
/// Returns `Some(Opportunity)` if a profitable arbitrage exists,
/// `None` otherwise.
///
/// # Example
///
/// ```ignore
/// use edgelord::domain::{detect_single_condition, DetectorConfig, OrderBookCache};
///
/// let config = DetectorConfig::default();
/// let cache = OrderBookCache::new();
/// // ... populate cache with orderbooks
///
/// if let Some(opp) = detect_single_condition(&pair, &cache, &config) {
///     println!("Found opportunity with edge: {}", opp.edge());
/// }
/// ```
pub fn detect_single_condition(
    pair: &MarketPair,
    cache: &OrderBookCache,
    config: &DetectorConfig,
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
            MarketId::new("test-market"),
            "Test question?",
            TokenId::new("yes-token"),
            TokenId::new("no-token"),
        )
    }

    fn make_config() -> DetectorConfig {
        DetectorConfig {
            min_edge: dec!(0.05),
            min_profit: dec!(0.50),
        }
    }

    #[test]
    fn test_detects_arbitrage_when_sum_below_one() {
        let pair = make_pair();
        let cache = OrderBookCache::new();
        let config = make_config();

        let yes_book = OrderBook::with_levels(
            pair.yes_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.40), dec!(100))],
        );
        let no_book = OrderBook::with_levels(
            pair.no_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(100))],
        );

        cache.update(yes_book);
        cache.update(no_book);

        let opp = detect_single_condition(&pair, &cache, &config);
        assert!(opp.is_some());

        let opp = opp.unwrap();
        assert_eq!(opp.edge(), dec!(0.10));
        assert_eq!(opp.total_cost(), dec!(0.90));
        assert_eq!(opp.expected_profit(), dec!(10.00));
    }

    #[test]
    fn test_no_arbitrage_when_sum_equals_one() {
        let pair = make_pair();
        let cache = OrderBookCache::new();
        let config = make_config();

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

        assert!(detect_single_condition(&pair, &cache, &config).is_none());
    }

    #[test]
    fn test_no_arbitrage_when_edge_too_small() {
        let pair = make_pair();
        let cache = OrderBookCache::new();
        let config = make_config();

        cache.update(OrderBook::with_levels(
            pair.yes_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.48), dec!(100))],
        ));
        cache.update(OrderBook::with_levels(
            pair.no_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(100))],
        ));

        assert!(detect_single_condition(&pair, &cache, &config).is_none());
    }

    #[test]
    fn test_no_arbitrage_when_profit_too_small() {
        let pair = make_pair();
        let cache = OrderBookCache::new();
        let config = make_config();

        cache.update(OrderBook::with_levels(
            pair.yes_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.40), dec!(1))],
        ));
        cache.update(OrderBook::with_levels(
            pair.no_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(1))],
        ));

        assert!(detect_single_condition(&pair, &cache, &config).is_none());
    }

    #[test]
    fn test_volume_limited_by_smaller_side() {
        let pair = make_pair();
        let cache = OrderBookCache::new();
        let config = make_config();

        cache.update(OrderBook::with_levels(
            pair.yes_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.40), dec!(50))],
        ));
        cache.update(OrderBook::with_levels(
            pair.no_token().clone(),
            vec![],
            vec![PriceLevel::new(dec!(0.50), dec!(100))],
        ));

        let opp = detect_single_condition(&pair, &cache, &config).unwrap();
        assert_eq!(opp.volume(), dec!(50));
        assert_eq!(opp.expected_profit(), dec!(5.00));
    }
}
```

**Step 2: Run cargo test**

**Step 3: Commit**

```bash
git add src/domain/detector.rs
git commit -m "refactor: update detector to use new domain types"
```

---

## Task 11: Update MarketRegistry and Client

**Files:**
- Modify: `src/polymarket/registry.rs`
- Modify: `src/polymarket/client.rs`

**Step 1: Update registry to use new MarketPair constructor**

**Step 2: Update client return type for ExchangeClient trait**

**Step 3: Run cargo check and tests**

**Step 4: Commit**

```bash
git add src/polymarket/registry.rs src/polymarket/client.rs
git commit -m "refactor: update polymarket module for new domain types"
```

---

## Task 12: Add Feature Flags

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add feature configuration**

```toml
[features]
default = ["polymarket"]
polymarket = ["dep:polymarket-client-sdk", "dep:alloy-signer-local"]

[dependencies]
# ... existing deps ...

# Polymarket CLOB client (optional)
polymarket-client-sdk = { version = "0.4", features = ["clob"], optional = true }
alloy-signer-local = { version = "1", optional = true }
```

**Step 2: Add cfg attributes to polymarket module**

```rust
// In lib.rs
#[cfg(feature = "polymarket")]
pub mod polymarket;
```

**Step 3: Run cargo check with default and without polymarket**

**Step 4: Commit**

```bash
git add Cargo.toml src/lib.rs
git commit -m "feat: add feature flags for exchange support"
```

---

## Task 13: Final Cleanup and Documentation

**Files:**
- All files - remove unnecessary `#[allow(dead_code)]`
- Add module-level documentation
- Run clippy and fix warnings

**Step 1: Remove all unnecessary `#[allow(dead_code)]` attributes**

**Step 2: Add/improve module documentation**

**Step 3: Run cargo clippy --all-features and fix**

**Step 4: Run cargo doc --all-features and verify**

**Step 5: Run cargo test**

**Step 6: Commit**

```bash
git add .
git commit -m "chore: final cleanup and documentation"
```

---

## Task 14: Delete Old Types File

**Files:**
- Delete: `src/domain/types.rs` (contents moved to id.rs, money.rs, market.rs)

**Step 1: Ensure all types are properly moved**

**Step 2: Delete types.rs**

**Step 3: Run cargo check and test**

**Step 4: Commit**

```bash
git rm src/domain/types.rs
git commit -m "chore: remove old types.rs (contents migrated)"
```

---

## Verification Checklist

Before marking complete:

- [ ] `cargo check --all-features` passes
- [ ] `cargo test` passes (all existing + new tests)
- [ ] `cargo clippy --all-features` passes with no warnings
- [ ] `cargo doc --all-features` generates without warnings
- [ ] No `#[allow(dead_code)]` on public items
- [ ] All modules have documentation
- [ ] lib.rs has crate-level documentation with example
- [ ] Exchange traits are properly abstracted
- [ ] Domain has no polymarket imports
- [ ] Feature flags work (`--no-default-features` builds domain only)

---

## Summary of Changes

| Before | After |
|--------|-------|
| `executor/` at top level | `polymarket/executor.rs` |
| `domain/` imports polymarket | Clean separation |
| `types.rs` monolith | `id.rs`, `money.rs`, `market.rs` |
| `pub` fields everywhere | Private fields + accessors |
| `Error::Execution(String)` | Structured `ExecutionError` |
| No lib.rs | Proper library structure |
| No traits | `ExchangeClient`, `OrderExecutor` |
| Scattered `#[allow(dead_code)]` | Clean, intentional visibility |
