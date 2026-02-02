# Phase 2: Detection Implementation Plan

> **Status:** ✅ COMPLETE

**Goal:** Build an OrderBook cache and detect single-condition arbitrage opportunities (YES + NO < $1.00) in real-time.

**Architecture:** OrderBook cache stores latest state per token. On each WebSocket update, cache updates and detector scans for arbitrage. Opportunities logged with full details. Market metadata maps token pairs to their parent market.

**Tech Stack:** Rust, rust_decimal for precise math, parking_lot for fast locks, std::collections::HashMap for cache

**Note:** This plan established the initial detection logic which was later refined in the comprehensive restructure with proper encapsulation, builder patterns, and trait abstractions.

---

## Task 1: Create Core Types Module

**Files:**
- Create: `src/types.rs`
- Modify: `src/main.rs`

**Step 1: Create src/types.rs with core domain types**

```rust
use rust_decimal::Decimal;
use std::fmt;

/// Token identifier - newtype for type safety
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TokenId(pub String);

impl fmt::Display for TokenId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for TokenId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for TokenId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Market condition identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MarketId(pub String);

impl fmt::Display for MarketId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for MarketId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Price and volume use Decimal for precision
pub type Price = Decimal;
pub type Volume = Decimal;

/// A single price level in the order book
#[derive(Debug, Clone)]
pub struct PriceLevel {
    pub price: Price,
    pub size: Volume,
}

/// Order book for a single token
#[derive(Debug, Clone)]
pub struct OrderBook {
    pub token_id: TokenId,
    pub bids: Vec<PriceLevel>,  // Sorted descending by price (best first)
    pub asks: Vec<PriceLevel>,  // Sorted ascending by price (best first)
}

impl OrderBook {
    pub fn new(token_id: TokenId) -> Self {
        Self {
            token_id,
            bids: Vec::new(),
            asks: Vec::new(),
        }
    }

    /// Best bid (highest buy price)
    pub fn best_bid(&self) -> Option<&PriceLevel> {
        self.bids.first()
    }

    /// Best ask (lowest sell price)
    pub fn best_ask(&self) -> Option<&PriceLevel> {
        self.asks.first()
    }
}

/// A YES/NO market pair
#[derive(Debug, Clone)]
pub struct MarketPair {
    pub market_id: MarketId,
    pub question: String,
    pub yes_token: TokenId,
    pub no_token: TokenId,
}

/// Detected arbitrage opportunity
#[derive(Debug, Clone)]
pub struct Opportunity {
    pub market_id: MarketId,
    pub question: String,
    pub yes_token: TokenId,
    pub no_token: TokenId,
    pub yes_ask: Price,
    pub no_ask: Price,
    pub total_cost: Price,
    pub edge: Price,
    pub volume: Volume,
    pub expected_profit: Price,
}
```

**Step 2: Update src/main.rs to declare module**

Add after the other mod declarations:
```rust
mod types;
```

**Step 3: Verify it compiles**

Run: `cargo build`

**Step 4: Commit**

```bash
git add src/types.rs src/main.rs
git commit -m "feat(types): add core domain types for orderbook and opportunities"
```

---

## Task 2: Create OrderBook Cache

**Files:**
- Create: `src/orderbook/mod.rs`
- Create: `src/orderbook/cache.rs`
- Modify: `src/main.rs`

**Step 1: Create src/orderbook directory and cache.rs**

```rust
use parking_lot::RwLock;
use std::collections::HashMap;

use crate::types::{OrderBook, PriceLevel, TokenId};
use crate::websocket::BookMessage;

/// Thread-safe cache of order books
pub struct OrderBookCache {
    books: RwLock<HashMap<TokenId, OrderBook>>,
}

impl OrderBookCache {
    pub fn new() -> Self {
        Self {
            books: RwLock::new(HashMap::new()),
        }
    }

    /// Update order book from WebSocket message
    pub fn update(&self, msg: &BookMessage) {
        let token_id = TokenId::from(msg.asset_id.clone());

        let bids: Vec<PriceLevel> = msg
            .bids
            .iter()
            .filter_map(|pl| {
                Some(PriceLevel {
                    price: pl.price.parse().ok()?,
                    size: pl.size.parse().ok()?,
                })
            })
            .collect();

        let asks: Vec<PriceLevel> = msg
            .asks
            .iter()
            .filter_map(|pl| {
                Some(PriceLevel {
                    price: pl.price.parse().ok()?,
                    size: pl.size.parse().ok()?,
                })
            })
            .collect();

        let book = OrderBook {
            token_id: token_id.clone(),
            bids,
            asks,
        };

        self.books.write().insert(token_id, book);
    }

    /// Get a snapshot of an order book
    pub fn get(&self, token_id: &TokenId) -> Option<OrderBook> {
        self.books.read().get(token_id).cloned()
    }

    /// Get snapshots of two order books atomically
    pub fn get_pair(&self, token_a: &TokenId, token_b: &TokenId) -> (Option<OrderBook>, Option<OrderBook>) {
        let books = self.books.read();
        (books.get(token_a).cloned(), books.get(token_b).cloned())
    }

    /// Number of books in cache
    pub fn len(&self) -> usize {
        self.books.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for OrderBookCache {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 2: Create src/orderbook/mod.rs**

```rust
mod cache;

pub use cache::OrderBookCache;
```

**Step 3: Add parking_lot to Cargo.toml**

Add under `[dependencies]`:
```toml
parking_lot = "0.12"
```

**Step 4: Update src/main.rs to declare module**

Add after the other mod declarations:
```rust
mod orderbook;
```

**Step 5: Verify it compiles**

Run: `cargo build`

**Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/orderbook/ src/main.rs
git commit -m "feat(orderbook): add thread-safe order book cache"
```

---

## Task 3: Create Market Registry

**Files:**
- Create: `src/orderbook/registry.rs`
- Modify: `src/orderbook/mod.rs`

**Step 1: Create src/orderbook/registry.rs**

```rust
use std::collections::HashMap;

use crate::api::Market;
use crate::types::{MarketId, MarketPair, TokenId};

/// Registry mapping tokens to their market pairs
pub struct MarketRegistry {
    /// Token ID -> Market pair it belongs to
    token_to_market: HashMap<TokenId, MarketPair>,
    /// All market pairs
    pairs: Vec<MarketPair>,
}

impl MarketRegistry {
    pub fn new() -> Self {
        Self {
            token_to_market: HashMap::new(),
            pairs: Vec::new(),
        }
    }

    /// Build registry from API market data
    /// Only includes 2-outcome (YES/NO) markets
    pub fn from_markets(markets: &[Market]) -> Self {
        let mut registry = Self::new();

        for market in markets {
            // Only handle 2-outcome markets for single-condition arbitrage
            if market.tokens.len() != 2 {
                continue;
            }

            // Find YES and NO tokens
            let yes_token = market.tokens.iter().find(|t| t.outcome.to_lowercase() == "yes");
            let no_token = market.tokens.iter().find(|t| t.outcome.to_lowercase() == "no");

            if let (Some(yes), Some(no)) = (yes_token, no_token) {
                let pair = MarketPair {
                    market_id: MarketId::from(market.condition_id.clone()),
                    question: market.question.clone().unwrap_or_default(),
                    yes_token: TokenId::from(yes.token_id.clone()),
                    no_token: TokenId::from(no.token_id.clone()),
                };

                registry.token_to_market.insert(pair.yes_token.clone(), pair.clone());
                registry.token_to_market.insert(pair.no_token.clone(), pair.clone());
                registry.pairs.push(pair);
            }
        }

        registry
    }

    /// Get the market pair for a token
    pub fn get_market_for_token(&self, token_id: &TokenId) -> Option<&MarketPair> {
        self.token_to_market.get(token_id)
    }

    /// Get all market pairs
    pub fn pairs(&self) -> &[MarketPair] {
        &self.pairs
    }

    /// Number of registered pairs
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }
}

impl Default for MarketRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 2: Update src/orderbook/mod.rs**

```rust
mod cache;
mod registry;

pub use cache::OrderBookCache;
pub use registry::MarketRegistry;
```

**Step 3: Verify it compiles**

Run: `cargo build`

**Step 4: Commit**

```bash
git add src/orderbook/
git commit -m "feat(orderbook): add market registry for token-to-market mapping"
```

---

## Task 4: Create Detector Module with Config

**Files:**
- Create: `src/detector/mod.rs`
- Create: `src/detector/config.rs`
- Modify: `src/main.rs`
- Modify: `src/config.rs`
- Modify: `config.toml`

**Step 1: Create src/detector/config.rs**

```rust
use rust_decimal::Decimal;
use serde::Deserialize;

/// Configuration for the arbitrage detector
#[derive(Debug, Clone, Deserialize)]
pub struct DetectorConfig {
    /// Minimum edge (profit per $1) to consider an opportunity
    /// e.g., 0.05 means YES + NO must be <= $0.95
    #[serde(default = "default_min_edge")]
    pub min_edge: Decimal,

    /// Minimum expected profit in dollars to act on
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
```

**Step 2: Create src/detector/mod.rs**

```rust
mod config;

pub use config::DetectorConfig;
```

**Step 3: Update src/main.rs to declare module**

Add after the other mod declarations:
```rust
mod detector;
```

**Step 4: Update src/config.rs to include detector config**

Add to the Config struct:
```rust
use crate::detector::DetectorConfig;
```

And add the field:
```rust
#[derive(Debug, Deserialize)]
pub struct Config {
    pub network: NetworkConfig,
    pub logging: LoggingConfig,
    #[serde(default)]
    pub detector: DetectorConfig,
}
```

**Step 5: Update config.toml**

Add at the end:
```toml
[detector]
min_edge = 0.05
min_profit = 0.50
```

**Step 6: Verify it compiles**

Run: `cargo build`

**Step 7: Commit**

```bash
git add src/detector/ src/config.rs src/main.rs config.toml
git commit -m "feat(detector): add detector config with min_edge and min_profit"
```

---

## Task 5: Implement Single-Condition Detector

**Files:**
- Create: `src/detector/single.rs`
- Modify: `src/detector/mod.rs`

**Step 1: Create src/detector/single.rs**

```rust
use rust_decimal::Decimal;

use crate::orderbook::{MarketRegistry, OrderBookCache};
use crate::types::{MarketPair, Opportunity, Price, Volume};

use super::DetectorConfig;

/// Detect single-condition arbitrage (YES + NO < $1.00)
pub fn detect_single_condition(
    pair: &MarketPair,
    cache: &OrderBookCache,
    config: &DetectorConfig,
) -> Option<Opportunity> {
    // Get both order books atomically
    let (yes_book, no_book) = cache.get_pair(&pair.yes_token, &pair.no_token);

    let yes_book = yes_book?;
    let no_book = no_book?;

    // Get best asks (what we'd pay to buy)
    let yes_ask = yes_book.best_ask()?;
    let no_ask = no_book.best_ask()?;

    // Calculate edge
    let total_cost = yes_ask.price + no_ask.price;
    let one = Decimal::ONE;

    // If total cost >= $1, no arbitrage
    if total_cost >= one {
        return None;
    }

    let edge = one - total_cost;

    // Check minimum edge
    if edge < config.min_edge {
        return None;
    }

    // Volume is limited by smaller side
    let volume = yes_ask.size.min(no_ask.size);

    // Expected profit
    let expected_profit = edge * volume;

    // Check minimum profit
    if expected_profit < config.min_profit {
        return None;
    }

    Some(Opportunity {
        market_id: pair.market_id.clone(),
        question: pair.question.clone(),
        yes_token: pair.yes_token.clone(),
        no_token: pair.no_token.clone(),
        yes_ask: yes_ask.price,
        no_ask: no_ask.price,
        total_cost,
        edge,
        volume,
        expected_profit,
    })
}

/// Scan all markets for arbitrage opportunities
pub fn scan_all(
    registry: &MarketRegistry,
    cache: &OrderBookCache,
    config: &DetectorConfig,
) -> Vec<Opportunity> {
    registry
        .pairs()
        .iter()
        .filter_map(|pair| detect_single_condition(pair, cache, config))
        .collect()
}
```

**Step 2: Update src/detector/mod.rs**

```rust
mod config;
mod single;

pub use config::DetectorConfig;
pub use single::{detect_single_condition, scan_all};
```

**Step 3: Verify it compiles**

Run: `cargo build`

**Step 4: Commit**

```bash
git add src/detector/
git commit -m "feat(detector): implement single-condition arbitrage detection"
```

---

## Task 6: Wire Detection into Main Loop

**Files:**
- Modify: `src/main.rs`

**Step 1: Replace src/main.rs with integrated detection**

```rust
mod api;
mod config;
mod detector;
mod error;
mod orderbook;
mod types;
mod websocket;

use std::sync::Arc;

use api::ApiClient;
use config::Config;
use detector::{scan_all, DetectorConfig};
use orderbook::{MarketRegistry, OrderBookCache};
use tokio::signal;
use tracing::{error, info, warn};
use websocket::{WebSocketHandler, WsMessage};

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    let config = match Config::load("config.toml") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    };

    config.init_logging();

    info!("edgelord starting");

    tokio::select! {
        result = run(config) => {
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

async fn run(config: Config) -> error::Result<()> {
    // Fetch active markets
    let api = ApiClient::new(config.network.api_url.clone());
    let markets = api.get_active_markets(20).await?;

    if markets.is_empty() {
        warn!("No active markets found");
        return Ok(());
    }

    // Build market registry (only YES/NO pairs)
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

    // Log the pairs we're tracking
    for pair in registry.pairs() {
        info!(
            market_id = %pair.market_id,
            question = %pair.question,
            "Tracking market"
        );
    }

    // Collect all token IDs to subscribe to
    let token_ids: Vec<String> = registry
        .pairs()
        .iter()
        .flat_map(|p| vec![p.yes_token.0.clone(), p.no_token.0.clone()])
        .collect();

    info!(tokens = token_ids.len(), "Subscribing to tokens");

    // Create shared state
    let cache = Arc::new(OrderBookCache::new());
    let registry = Arc::new(registry);
    let detector_config = Arc::new(config.detector.clone());

    // Connect to WebSocket and process messages
    let handler = WebSocketHandler::new(config.network.ws_url);

    let cache_clone = cache.clone();
    let registry_clone = registry.clone();
    let detector_config_clone = detector_config.clone();

    handler
        .run(token_ids, move |msg| {
            handle_message(
                msg,
                &cache_clone,
                &registry_clone,
                &detector_config_clone,
            );
        })
        .await?;

    Ok(())
}

fn handle_message(
    msg: WsMessage,
    cache: &OrderBookCache,
    registry: &MarketRegistry,
    config: &DetectorConfig,
) {
    match msg {
        WsMessage::Book(book) => {
            // Update cache
            cache.update(&book);

            // Get the market for this token
            let token_id = types::TokenId::from(book.asset_id.clone());
            if let Some(pair) = registry.get_market_for_token(&token_id) {
                // Check for arbitrage on this market
                if let Some(opp) = detector::detect_single_condition(pair, cache, config) {
                    info!(
                        market = %opp.market_id,
                        question = %opp.question,
                        yes_ask = %opp.yes_ask,
                        no_ask = %opp.no_ask,
                        total_cost = %opp.total_cost,
                        edge = %opp.edge,
                        volume = %opp.volume,
                        expected_profit = %opp.expected_profit,
                        "ARBITRAGE DETECTED"
                    );
                }
            }
        }
        WsMessage::PriceChange(_) => {
            // Price changes don't have full book data, ignore for now
        }
        _ => {}
    }
}
```

**Step 2: Verify it compiles**

Run: `cargo build`

**Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire detection into WebSocket message loop"
```

---

## Task 7: Add Unit Tests for Detector

**Files:**
- Modify: `src/detector/single.rs`

**Step 1: Add tests module to src/detector/single.rs**

Add at the end of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MarketId, OrderBook, PriceLevel, TokenId};
    use rust_decimal_macros::dec;

    fn make_pair() -> MarketPair {
        MarketPair {
            market_id: MarketId::from("test-market"),
            question: "Test question?".to_string(),
            yes_token: TokenId::from("yes-token"),
            no_token: TokenId::from("no-token"),
        }
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

        // YES ask: 0.40, NO ask: 0.50 -> total 0.90, edge 0.10
        let yes_book = OrderBook {
            token_id: pair.yes_token.clone(),
            bids: vec![],
            asks: vec![PriceLevel {
                price: dec!(0.40),
                size: dec!(100),
            }],
        };
        let no_book = OrderBook {
            token_id: pair.no_token.clone(),
            bids: vec![],
            asks: vec![PriceLevel {
                price: dec!(0.50),
                size: dec!(100),
            }],
        };

        cache.books.write().insert(pair.yes_token.clone(), yes_book);
        cache.books.write().insert(pair.no_token.clone(), no_book);

        let opp = detect_single_condition(&pair, &cache, &config);
        assert!(opp.is_some());

        let opp = opp.unwrap();
        assert_eq!(opp.edge, dec!(0.10));
        assert_eq!(opp.total_cost, dec!(0.90));
        assert_eq!(opp.expected_profit, dec!(10.00)); // 0.10 * 100
    }

    #[test]
    fn test_no_arbitrage_when_sum_equals_one() {
        let pair = make_pair();
        let cache = OrderBookCache::new();
        let config = make_config();

        // YES ask: 0.50, NO ask: 0.50 -> total 1.00, no edge
        let yes_book = OrderBook {
            token_id: pair.yes_token.clone(),
            bids: vec![],
            asks: vec![PriceLevel {
                price: dec!(0.50),
                size: dec!(100),
            }],
        };
        let no_book = OrderBook {
            token_id: pair.no_token.clone(),
            bids: vec![],
            asks: vec![PriceLevel {
                price: dec!(0.50),
                size: dec!(100),
            }],
        };

        cache.books.write().insert(pair.yes_token.clone(), yes_book);
        cache.books.write().insert(pair.no_token.clone(), no_book);

        let opp = detect_single_condition(&pair, &cache, &config);
        assert!(opp.is_none());
    }

    #[test]
    fn test_no_arbitrage_when_edge_too_small() {
        let pair = make_pair();
        let cache = OrderBookCache::new();
        let config = make_config(); // min_edge = 0.05

        // YES ask: 0.48, NO ask: 0.50 -> total 0.98, edge 0.02 (below min)
        let yes_book = OrderBook {
            token_id: pair.yes_token.clone(),
            bids: vec![],
            asks: vec![PriceLevel {
                price: dec!(0.48),
                size: dec!(100),
            }],
        };
        let no_book = OrderBook {
            token_id: pair.no_token.clone(),
            bids: vec![],
            asks: vec![PriceLevel {
                price: dec!(0.50),
                size: dec!(100),
            }],
        };

        cache.books.write().insert(pair.yes_token.clone(), yes_book);
        cache.books.write().insert(pair.no_token.clone(), no_book);

        let opp = detect_single_condition(&pair, &cache, &config);
        assert!(opp.is_none());
    }

    #[test]
    fn test_no_arbitrage_when_profit_too_small() {
        let pair = make_pair();
        let cache = OrderBookCache::new();
        let config = make_config(); // min_profit = 0.50

        // YES ask: 0.40, NO ask: 0.50 -> edge 0.10, but volume only 1
        // expected profit = 0.10 * 1 = 0.10 (below min)
        let yes_book = OrderBook {
            token_id: pair.yes_token.clone(),
            bids: vec![],
            asks: vec![PriceLevel {
                price: dec!(0.40),
                size: dec!(1),
            }],
        };
        let no_book = OrderBook {
            token_id: pair.no_token.clone(),
            bids: vec![],
            asks: vec![PriceLevel {
                price: dec!(0.50),
                size: dec!(1),
            }],
        };

        cache.books.write().insert(pair.yes_token.clone(), yes_book);
        cache.books.write().insert(pair.no_token.clone(), no_book);

        let opp = detect_single_condition(&pair, &cache, &config);
        assert!(opp.is_none());
    }

    #[test]
    fn test_volume_limited_by_smaller_side() {
        let pair = make_pair();
        let cache = OrderBookCache::new();
        let config = make_config();

        // YES has 50 volume, NO has 100 volume -> should use 50
        let yes_book = OrderBook {
            token_id: pair.yes_token.clone(),
            bids: vec![],
            asks: vec![PriceLevel {
                price: dec!(0.40),
                size: dec!(50),
            }],
        };
        let no_book = OrderBook {
            token_id: pair.no_token.clone(),
            bids: vec![],
            asks: vec![PriceLevel {
                price: dec!(0.50),
                size: dec!(100),
            }],
        };

        cache.books.write().insert(pair.yes_token.clone(), yes_book);
        cache.books.write().insert(pair.no_token.clone(), no_book);

        let opp = detect_single_condition(&pair, &cache, &config);
        assert!(opp.is_some());

        let opp = opp.unwrap();
        assert_eq!(opp.volume, dec!(50));
        assert_eq!(opp.expected_profit, dec!(5.00)); // 0.10 * 50
    }
}
```

**Step 2: Add rust_decimal_macros to Cargo.toml dev-dependencies**

```toml
[dev-dependencies]
tokio-test = "0.4"
rust_decimal_macros = "1"
```

**Step 3: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock src/detector/single.rs
git commit -m "test(detector): add unit tests for single-condition detection"
```

---

## Task 8: Final Verification and Cleanup

**Step 1: Run clippy**

Run: `cargo clippy`

**Step 2: Run fmt**

Run: `cargo fmt`

**Step 3: Run all tests**

Run: `cargo test`

**Step 4: Commit any fixes**

```bash
git add -A
git commit -m "chore: format and lint fixes" --allow-empty
```

**Step 5: Push the branch**

```bash
git push -u origin feat/phase-2-detection
```

---

## Summary

**Phase 2 Complete.** You now have:

- **Core types** (`src/types.rs`) - TokenId, MarketId, OrderBook, Opportunity
- **OrderBook cache** (`src/orderbook/cache.rs`) - Thread-safe storage with parking_lot
- **Market registry** (`src/orderbook/registry.rs`) - Maps tokens to YES/NO pairs
- **Detector config** (`src/detector/config.rs`) - min_edge, min_profit thresholds
- **Single-condition detector** (`src/detector/single.rs`) - Finds YES+NO < $1 opportunities
- **Integrated main loop** - Updates cache, runs detection on each message
- **Unit tests** - 5 tests covering detection logic

**Milestone Achieved:** Logs "ARBITRAGE DETECTED" when YES + NO asks sum to less than $0.95.

**Next Phase:** Phase 3 will add execution on testnet.

---

## Files Created/Modified

```
src/
├── main.rs              (modified)
├── config.rs            (modified)
├── types.rs             (new)
├── orderbook/
│   ├── mod.rs           (new)
│   ├── cache.rs         (new)
│   └── registry.rs      (new)
└── detector/
    ├── mod.rs           (new)
    ├── config.rs        (new)
    └── single.rs        (new)

config.toml              (modified)
Cargo.toml               (modified)
```
