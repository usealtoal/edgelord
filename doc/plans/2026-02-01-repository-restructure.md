# Repository Restructure Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Restructure the codebase into `polymarket/` (exchange-specific) and `domain/` (exchange-agnostic) modules for a clean, professional Rust project.

**Architecture:** Move all Polymarket-specific code (REST client, WebSocket, API types, market registry) under `polymarket/`. Move domain types and detection logic under `domain/`. Flatten file structure where single files suffice.

**Tech Stack:** Rust, existing dependencies unchanged

---

## Target Structure

```
src/
├── polymarket/           # All Polymarket-specific code
│   ├── mod.rs
│   ├── client.rs         # REST API client
│   ├── websocket.rs      # WebSocket handler
│   ├── messages.rs       # WebSocket message types
│   ├── types.rs          # API response types (Market, Token, etc.)
│   └── registry.rs       # MarketRegistry (Polymarket YES/NO structure)
│
├── domain/               # Exchange-agnostic business logic
│   ├── mod.rs
│   ├── types.rs          # TokenId, MarketId, OrderBook, Opportunity, etc.
│   ├── orderbook.rs      # OrderBookCache
│   └── detector.rs       # Detection logic + config
│
├── config.rs             # App configuration
├── error.rs              # Error types
└── main.rs               # Entry point
```

---

## Task 1: Create polymarket/ Module Structure

**Files:**
- Create: `src/polymarket/mod.rs`
- Create: `src/polymarket/types.rs`
- Create: `src/polymarket/client.rs`
- Delete: `src/api/` (after moving content)

**Step 1: Create src/polymarket/ directory and mod.rs**

```bash
mkdir -p src/polymarket
```

**Step 2: Create src/polymarket/types.rs**

```rust
//! Polymarket API response types.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct MarketsResponse {
    pub data: Option<Vec<Market>>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Market {
    pub condition_id: String,
    pub question: Option<String>,
    pub tokens: Vec<Token>,
    pub active: bool,
    pub closed: bool,
}

#[derive(Debug, Deserialize)]
pub struct Token {
    pub token_id: String,
    pub outcome: String,
    pub price: Option<f64>,
}

impl Market {
    pub fn token_ids(&self) -> Vec<String> {
        self.tokens.iter().map(|t| t.token_id.clone()).collect()
    }
}
```

**Step 3: Create src/polymarket/client.rs**

```rust
//! Polymarket REST API client.

use reqwest::Client;
use tracing::{debug, info};

use super::types::{Market, MarketsResponse};
use crate::error::Result;

pub struct PolymarketClient {
    client: Client,
    base_url: String,
}

impl PolymarketClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    /// Fetch active markets, limited to a reasonable number for initial testing
    pub async fn get_active_markets(&self, limit: usize) -> Result<Vec<Market>> {
        let url = format!(
            "{}/markets?active=true&closed=false&limit={}",
            self.base_url, limit
        );

        info!(url = %url, "Fetching active markets");

        let response: MarketsResponse = self.client.get(&url).send().await?.json().await?;

        let markets = response.data.unwrap_or_default();
        debug!(count = markets.len(), "Fetched markets");

        Ok(markets)
    }
}
```

**Step 4: Create src/polymarket/mod.rs (partial - will add more later)**

```rust
//! Polymarket exchange integration.

mod client;
mod types;

pub use client::PolymarketClient;
pub use types::{Market, Token};
```

**Step 5: Verify it compiles (will have errors until main.rs is updated)**

Run: `source /Users/rdekovich/.cargo/env && cargo check 2>&1 | head -20`

**Step 6: Commit**

```bash
git add src/polymarket/
git commit -m "refactor(polymarket): create polymarket module with client and types"
```

---

## Task 2: Move WebSocket to polymarket/

**Files:**
- Create: `src/polymarket/websocket.rs`
- Create: `src/polymarket/messages.rs`
- Modify: `src/polymarket/mod.rs`
- Delete: `src/websocket/` (after moving content)

**Step 1: Create src/polymarket/messages.rs**

```rust
//! Polymarket WebSocket message types.

use serde::{Deserialize, Serialize};

/// Subscription request sent to Polymarket WebSocket
#[derive(Debug, Serialize)]
pub struct SubscribeMessage {
    pub assets_ids: Vec<String>,
    #[serde(rename = "type")]
    pub msg_type: String,
}

impl SubscribeMessage {
    pub fn new(asset_ids: Vec<String>) -> Self {
        Self {
            assets_ids: asset_ids,
            msg_type: "market".into(),
        }
    }
}

/// Messages received from Polymarket WebSocket
#[derive(Debug, Deserialize)]
#[serde(tag = "event_type")]
pub enum WsMessage {
    #[serde(rename = "book")]
    Book(BookMessage),

    #[serde(rename = "price_change")]
    PriceChange(PriceChangeMessage),

    #[serde(rename = "tick_size_change")]
    TickSizeChange(serde_json::Value),

    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct BookMessage {
    pub asset_id: String,
    pub market: Option<String>,
    pub bids: Vec<WsPriceLevel>,
    pub asks: Vec<WsPriceLevel>,
    pub timestamp: Option<String>,
    pub hash: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PriceChangeMessage {
    pub asset_id: String,
    pub market: Option<String>,
    pub price: Option<String>,
    pub changes: Option<Vec<WsPriceLevel>>,
}

/// Price level as received from WebSocket (strings, not decimals)
#[derive(Debug, Clone, Deserialize)]
pub struct WsPriceLevel {
    pub price: String,
    pub size: String,
}
```

**Step 2: Create src/polymarket/websocket.rs**

```rust
//! Polymarket WebSocket handler.

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info, warn};

use super::messages::{SubscribeMessage, WsMessage};
use crate::error::Result;

pub struct WebSocketHandler {
    url: String,
}

impl WebSocketHandler {
    pub fn new(url: String) -> Self {
        Self { url }
    }

    pub async fn connect(&self) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>> {
        info!(url = %self.url, "Connecting to WebSocket");

        let (ws_stream, response) = connect_async(&self.url).await?;

        info!(
            status = %response.status(),
            "WebSocket connected"
        );

        Ok(ws_stream)
    }

    pub async fn subscribe(
        ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        asset_ids: Vec<String>,
    ) -> Result<()> {
        let msg = SubscribeMessage::new(asset_ids.clone());
        let json = serde_json::to_string(&msg)?;

        info!(assets = ?asset_ids, "Subscribing to assets");
        ws.send(Message::Text(json)).await?;

        Ok(())
    }

    pub async fn run<F>(&self, asset_ids: Vec<String>, mut on_message: F) -> Result<()>
    where
        F: FnMut(WsMessage),
    {
        let mut ws = self.connect().await?;

        Self::subscribe(&mut ws, asset_ids).await?;

        info!("Listening for messages...");

        while let Some(msg_result) = ws.next().await {
            match msg_result {
                Ok(Message::Text(text)) => {
                    debug!(raw = %text, "Received message");

                    match serde_json::from_str::<WsMessage>(&text) {
                        Ok(ws_msg) => on_message(ws_msg),
                        Err(e) => {
                            warn!(
                                error = %e,
                                raw = %text,
                                "Failed to parse message"
                            );
                        }
                    }
                }
                Ok(Message::Ping(data)) => {
                    debug!("Received ping");
                    ws.send(Message::Pong(data)).await?;
                }
                Ok(Message::Close(frame)) => {
                    info!(frame = ?frame, "WebSocket closed by server");
                    break;
                }
                Ok(_) => {}
                Err(e) => {
                    error!(error = %e, "WebSocket error");
                    break;
                }
            }
        }

        Ok(())
    }
}
```

**Step 3: Update src/polymarket/mod.rs**

```rust
//! Polymarket exchange integration.

mod client;
mod messages;
mod types;
mod websocket;

pub use client::PolymarketClient;
pub use messages::{BookMessage, WsMessage, WsPriceLevel};
pub use types::{Market, Token};
pub use websocket::WebSocketHandler;
```

**Step 4: Commit**

```bash
git add src/polymarket/
git commit -m "refactor(polymarket): move websocket handler and messages"
```

---

## Task 3: Create domain/ Module with Types

**Files:**
- Create: `src/domain/mod.rs`
- Create: `src/domain/types.rs`
- Delete: `src/types.rs` (after moving content)

**Step 1: Create src/domain/ directory**

```bash
mkdir -p src/domain
```

**Step 2: Create src/domain/types.rs**

```rust
//! Core domain types for arbitrage detection.

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
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
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

**Step 3: Create src/domain/mod.rs (partial)**

```rust
//! Exchange-agnostic domain logic.

mod types;

pub use types::{MarketId, MarketPair, Opportunity, OrderBook, PriceLevel, TokenId};
```

**Step 4: Commit**

```bash
git add src/domain/
git commit -m "refactor(domain): create domain module with core types"
```

---

## Task 4: Move OrderBookCache to domain/

**Files:**
- Create: `src/domain/orderbook.rs`
- Modify: `src/domain/mod.rs`
- Delete: `src/orderbook/cache.rs` (after verifying)

**Step 1: Create src/domain/orderbook.rs**

```rust
//! Thread-safe order book cache.

use parking_lot::RwLock;
use std::collections::HashMap;

use super::types::{OrderBook, PriceLevel, TokenId};
use crate::polymarket::{BookMessage, WsPriceLevel};

/// Thread-safe cache of order books
pub struct OrderBookCache {
    pub(crate) books: RwLock<HashMap<TokenId, OrderBook>>,
}

impl OrderBookCache {
    pub fn new() -> Self {
        Self {
            books: RwLock::new(HashMap::new()),
        }
    }

    /// Update order book from WebSocket message
    pub fn update_from_ws(&self, msg: &BookMessage) {
        let token_id = TokenId::from(msg.asset_id.clone());

        let bids = Self::parse_levels(&msg.bids);
        let asks = Self::parse_levels(&msg.asks);

        let book = OrderBook {
            token_id: token_id.clone(),
            bids,
            asks,
        };

        self.books.write().insert(token_id, book);
    }

    fn parse_levels(levels: &[WsPriceLevel]) -> Vec<PriceLevel> {
        levels
            .iter()
            .filter_map(|pl| {
                Some(PriceLevel {
                    price: pl.price.parse().ok()?,
                    size: pl.size.parse().ok()?,
                })
            })
            .collect()
    }

    /// Get a snapshot of an order book
    pub fn get(&self, token_id: &TokenId) -> Option<OrderBook> {
        self.books.read().get(token_id).cloned()
    }

    /// Get snapshots of two order books atomically
    pub fn get_pair(
        &self,
        token_a: &TokenId,
        token_b: &TokenId,
    ) -> (Option<OrderBook>, Option<OrderBook>) {
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

**Step 2: Update src/domain/mod.rs**

```rust
//! Exchange-agnostic domain logic.

mod orderbook;
mod types;

pub use orderbook::OrderBookCache;
pub use types::{MarketId, MarketPair, Opportunity, OrderBook, PriceLevel, TokenId};
```

**Step 3: Commit**

```bash
git add src/domain/
git commit -m "refactor(domain): move orderbook cache to domain module"
```

---

## Task 5: Move Detector to domain/

**Files:**
- Create: `src/domain/detector.rs`
- Modify: `src/domain/mod.rs`
- Delete: `src/detector/` (after verifying)

**Step 1: Create src/domain/detector.rs**

```rust
//! Arbitrage detection logic.

use rust_decimal::Decimal;
use serde::Deserialize;

use super::{MarketPair, Opportunity, OrderBookCache};

/// Configuration for the arbitrage detector
#[derive(Debug, Clone, Deserialize)]
pub struct DetectorConfig {
    /// Minimum edge (profit per $1) to consider an opportunity
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

/// Detect single-condition arbitrage (YES + NO < $1.00)
pub fn detect_single_condition(
    pair: &MarketPair,
    cache: &OrderBookCache,
    config: &DetectorConfig,
) -> Option<Opportunity> {
    let (yes_book, no_book) = cache.get_pair(&pair.yes_token, &pair.no_token);

    let yes_book = yes_book?;
    let no_book = no_book?;

    let yes_ask = yes_book.best_ask()?;
    let no_ask = no_book.best_ask()?;

    let total_cost = yes_ask.price + no_ask.price;
    let one = Decimal::ONE;

    if total_cost >= one {
        return None;
    }

    let edge = one - total_cost;

    if edge < config.min_edge {
        return None;
    }

    let volume = yes_ask.size.min(no_ask.size);
    let expected_profit = edge * volume;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{OrderBook, PriceLevel, TokenId};
    use rust_decimal_macros::dec;

    fn make_pair() -> MarketPair {
        MarketPair {
            market_id: "test-market".to_string().into(),
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
        assert_eq!(opp.expected_profit, dec!(10.00));
    }

    #[test]
    fn test_no_arbitrage_when_sum_equals_one() {
        let pair = make_pair();
        let cache = OrderBookCache::new();
        let config = make_config();

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
        let config = make_config();

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
        let config = make_config();

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
        assert_eq!(opp.expected_profit, dec!(5.00));
    }
}
```

**Step 2: Update src/domain/mod.rs**

```rust
//! Exchange-agnostic domain logic.

mod detector;
mod orderbook;
mod types;

pub use detector::{detect_single_condition, DetectorConfig};
pub use orderbook::OrderBookCache;
pub use types::{MarketId, MarketPair, Opportunity, OrderBook, PriceLevel, TokenId};
```

**Step 3: Commit**

```bash
git add src/domain/
git commit -m "refactor(domain): move detector logic to domain module"
```

---

## Task 6: Move MarketRegistry to polymarket/

**Files:**
- Create: `src/polymarket/registry.rs`
- Modify: `src/polymarket/mod.rs`
- Delete: `src/orderbook/registry.rs` (after verifying)

**Step 1: Create src/polymarket/registry.rs**

```rust
//! Polymarket-specific market registry for YES/NO pairs.

use std::collections::HashMap;

use super::types::Market;
use crate::domain::{MarketId, MarketPair, TokenId};

/// Registry mapping tokens to their market pairs.
/// This is Polymarket-specific because it understands the YES/NO token structure.
pub struct MarketRegistry {
    token_to_market: HashMap<TokenId, MarketPair>,
    pairs: Vec<MarketPair>,
}

impl MarketRegistry {
    pub fn new() -> Self {
        Self {
            token_to_market: HashMap::new(),
            pairs: Vec::new(),
        }
    }

    /// Build registry from Polymarket API market data.
    /// Only includes 2-outcome (YES/NO) markets.
    pub fn from_markets(markets: &[Market]) -> Self {
        let mut registry = Self::new();

        for market in markets {
            if market.tokens.len() != 2 {
                continue;
            }

            let yes_token = market
                .tokens
                .iter()
                .find(|t| t.outcome.to_lowercase() == "yes");
            let no_token = market
                .tokens
                .iter()
                .find(|t| t.outcome.to_lowercase() == "no");

            if let (Some(yes), Some(no)) = (yes_token, no_token) {
                let pair = MarketPair {
                    market_id: MarketId::from(market.condition_id.clone()),
                    question: market.question.clone().unwrap_or_default(),
                    yes_token: TokenId::from(yes.token_id.clone()),
                    no_token: TokenId::from(no.token_id.clone()),
                };

                registry
                    .token_to_market
                    .insert(pair.yes_token.clone(), pair.clone());
                registry
                    .token_to_market
                    .insert(pair.no_token.clone(), pair.clone());
                registry.pairs.push(pair);
            }
        }

        registry
    }

    pub fn get_market_for_token(&self, token_id: &TokenId) -> Option<&MarketPair> {
        self.token_to_market.get(token_id)
    }

    pub fn pairs(&self) -> &[MarketPair] {
        &self.pairs
    }

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

**Step 2: Update src/polymarket/mod.rs**

```rust
//! Polymarket exchange integration.

mod client;
mod messages;
mod registry;
mod types;
mod websocket;

pub use client::PolymarketClient;
pub use messages::{BookMessage, WsMessage, WsPriceLevel};
pub use registry::MarketRegistry;
pub use types::{Market, Token};
pub use websocket::WebSocketHandler;
```

**Step 3: Commit**

```bash
git add src/polymarket/
git commit -m "refactor(polymarket): move market registry to polymarket module"
```

---

## Task 7: Update config.rs

**Files:**
- Modify: `src/config.rs`

**Step 1: Update imports in src/config.rs**

```rust
use serde::Deserialize;
use std::path::Path;
use tracing_subscriber::{fmt, EnvFilter};

use crate::domain::DetectorConfig;
use crate::error::{Error, Result};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub network: NetworkConfig,
    pub logging: LoggingConfig,
    #[serde(default)]
    pub detector: DetectorConfig,
}

#[derive(Debug, Deserialize)]
pub struct NetworkConfig {
    pub ws_url: String,
    pub api_url: String,
}

#[derive(Debug, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
}

impl Config {
    #[allow(clippy::result_large_err)]
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::Config(format!("Failed to read config file: {}", e)))?;

        let config: Config = toml::from_str(&content)
            .map_err(|e| Error::Config(format!("Failed to parse config: {}", e)))?;

        config.validate()?;

        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        if self.network.ws_url.is_empty() {
            return Err(Error::Config("ws_url cannot be empty".into()));
        }
        if self.network.api_url.is_empty() {
            return Err(Error::Config("api_url cannot be empty".into()));
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            network: NetworkConfig {
                ws_url: "wss://ws-subscriptions-clob.polymarket.com/ws/market".into(),
                api_url: "https://clob.polymarket.com".into(),
            },
            logging: LoggingConfig {
                level: "info".into(),
                format: "pretty".into(),
            },
            detector: DetectorConfig::default(),
        }
    }
}

impl Config {
    pub fn init_logging(&self) {
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(&self.logging.level));

        match self.logging.format.as_str() {
            "json" => {
                fmt().json().with_env_filter(filter).init();
            }
            _ => {
                fmt().with_env_filter(filter).init();
            }
        }
    }
}
```

**Step 2: Commit**

```bash
git add src/config.rs
git commit -m "refactor(config): update imports for new module structure"
```

---

## Task 8: Update main.rs

**Files:**
- Modify: `src/main.rs`

**Step 1: Replace src/main.rs with updated imports**

```rust
mod config;
mod domain;
mod error;
mod polymarket;

use std::sync::Arc;

use config::Config;
use domain::{detect_single_condition, DetectorConfig, OrderBookCache, TokenId};
use polymarket::{MarketRegistry, PolymarketClient, WebSocketHandler, WsMessage};
use tokio::signal;
use tracing::{error, info, warn};

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
        info!(
            market_id = %pair.market_id,
            question = %pair.question,
            "Tracking market"
        );
    }

    let token_ids: Vec<String> = registry
        .pairs()
        .iter()
        .flat_map(|p| vec![p.yes_token.0.clone(), p.no_token.0.clone()])
        .collect();

    info!(tokens = token_ids.len(), "Subscribing to tokens");

    let cache = Arc::new(OrderBookCache::new());
    let registry = Arc::new(registry);
    let detector_config = Arc::new(config.detector.clone());

    let handler = WebSocketHandler::new(config.network.ws_url);

    let cache_clone = cache.clone();
    let registry_clone = registry.clone();
    let detector_config_clone = detector_config.clone();

    handler
        .run(token_ids, move |msg| {
            handle_message(msg, &cache_clone, &registry_clone, &detector_config_clone);
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
            cache.update_from_ws(&book);

            let token_id = TokenId::from(book.asset_id.clone());
            if let Some(pair) = registry.get_market_for_token(&token_id) {
                if let Some(opp) = detect_single_condition(pair, cache, config) {
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
        WsMessage::PriceChange(_) => {}
        _ => {}
    }
}
```

**Step 2: Commit**

```bash
git add src/main.rs
git commit -m "refactor(main): update for new module structure"
```

---

## Task 9: Delete Old Directories

**Files:**
- Delete: `src/api/`
- Delete: `src/websocket/`
- Delete: `src/orderbook/`
- Delete: `src/detector/`
- Delete: `src/types.rs`

**Step 1: Run tests to verify everything works**

Run: `source /Users/rdekovich/.cargo/env && cargo test`

Expected: All 5 tests pass

**Step 2: Run clippy**

Run: `source /Users/rdekovich/.cargo/env && cargo clippy -- -D warnings`

**Step 3: Delete old directories**

```bash
rm -rf src/api src/websocket src/orderbook src/detector src/types.rs
```

**Step 4: Verify build still works**

Run: `source /Users/rdekovich/.cargo/env && cargo build`

**Step 5: Commit**

```bash
git add -A
git commit -m "refactor: remove old module structure"
```

---

## Task 10: Final Cleanup and Format

**Step 1: Run fmt**

Run: `source /Users/rdekovich/.cargo/env && cargo fmt`

**Step 2: Run clippy with fixes**

Run: `source /Users/rdekovich/.cargo/env && cargo clippy --fix --allow-dirty`

**Step 3: Run tests**

Run: `source /Users/rdekovich/.cargo/env && cargo test`

**Step 4: Commit any changes**

```bash
git add -A
git commit -m "chore: format and lint cleanup" --allow-empty
```

**Step 5: Push branch**

```bash
git push -u origin feat/repository-restructure
```

---

## Summary

**New Structure:**
```
src/
├── polymarket/           # Polymarket-specific (6 files)
│   ├── mod.rs
│   ├── client.rs
│   ├── websocket.rs
│   ├── messages.rs
│   ├── types.rs
│   └── registry.rs
│
├── domain/               # Exchange-agnostic (4 files)
│   ├── mod.rs
│   ├── types.rs
│   ├── orderbook.rs
│   └── detector.rs
│
├── config.rs
├── error.rs
└── main.rs
```

**Benefits:**
- Clear separation: Polymarket-specific vs exchange-agnostic
- Flatter structure: No unnecessary nesting
- Future-proof: Adding another exchange = new `kalshi/` directory
- Professional: Follows Rust community conventions

**Tests:** All 5 detector tests preserved and passing
