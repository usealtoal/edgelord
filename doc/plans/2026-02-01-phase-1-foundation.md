# Phase 1: Foundation Implementation Plan

> **Status:** ✅ COMPLETE

**Goal:** Connect to Polymarket WebSocket and display live market data in the terminal.

**Architecture:** Tokio async runtime with WebSocket client. Config loaded from TOML file with env var overrides. Structured logging via tracing. Clean separation between config, WebSocket handling, and message parsing.

**Tech Stack:** Rust, tokio, tokio-tungstenite, serde, tracing, dotenvy, toml

**Note:** This plan established the initial project structure which was later refined in the comprehensive restructure.

---

## Task 1: Initialize Cargo Project

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `.gitignore`

**Step 1: Create Cargo.toml**

```toml
[package]
name = "edgelord"
version = "0.1.0"
edition = "2021"
description = "Polymarket arbitrage detection and execution"
license = "MIT"

[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }

# WebSocket
tokio-tungstenite = { version = "0.24", features = ["native-tls"] }
futures-util = "0.3"

# HTTP (for REST API calls)
reqwest = { version = "0.12", features = ["json"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Decimal math
rust_decimal = { version = "1", features = ["serde"] }

# Configuration
dotenvy = "0.15"
toml = "0.8"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }

# Error handling
thiserror = "1"
anyhow = "1"

# Time
chrono = { version = "0.4", features = ["serde"] }

# URL parsing
url = "2"

[dev-dependencies]
tokio-test = "0.4"
```

**Step 2: Create minimal src/main.rs**

```rust
fn main() {
    println!("edgelord starting...");
}
```

**Step 3: Create .gitignore**

```
/target
.env
*.log
```

**Step 4: Verify project compiles**

Run: `cargo build`
Expected: Compiles successfully, downloads dependencies

**Step 5: Commit**

```bash
git add Cargo.toml src/main.rs .gitignore
git commit -m "chore: initialize cargo project with dependencies"
```

---

## Task 2: Create Error Types

**Files:**
- Create: `src/error.rs`
- Modify: `src/main.rs`

**Step 1: Create src/error.rs**

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),
}

pub type Result<T> = std::result::Result<T, Error>;
```

**Step 2: Update src/main.rs to declare module**

```rust
mod error;

fn main() {
    println!("edgelord starting...");
}
```

**Step 3: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/error.rs src/main.rs
git commit -m "feat(error): add error types with thiserror"
```

---

## Task 3: Create Configuration Module

**Files:**
- Create: `src/config.rs`
- Create: `config.toml`
- Create: `.env.example`
- Modify: `src/main.rs`

**Step 1: Create src/config.rs**

```rust
use serde::Deserialize;
use std::path::Path;

use crate::error::{Error, Result};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub network: NetworkConfig,
    pub logging: LoggingConfig,
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
        }
    }
}
```

**Step 2: Create config.toml**

```toml
[network]
ws_url = "wss://ws-subscriptions-clob.polymarket.com/ws/market"
api_url = "https://clob.polymarket.com"

[logging]
level = "info"
format = "pretty"
```

**Step 3: Create .env.example**

```bash
# Environment overrides (optional)
# RUST_LOG=debug
```

**Step 4: Update src/main.rs**

```rust
mod config;
mod error;

use config::Config;

fn main() {
    // Load environment variables from .env if present
    let _ = dotenvy::dotenv();

    let config = Config::load("config.toml").expect("Failed to load config");
    println!("Config loaded: {:?}", config);
}
```

**Step 5: Verify it runs**

Run: `cargo run`
Expected: Prints config struct

**Step 6: Commit**

```bash
git add src/config.rs config.toml .env.example src/main.rs
git commit -m "feat(config): add TOML configuration loading"
```

---

## Task 4: Add Structured Logging

**Files:**
- Modify: `src/main.rs`
- Modify: `src/config.rs`

**Step 1: Update src/config.rs to add logging setup**

Add this function at the end of the file:

```rust
use tracing_subscriber::{fmt, EnvFilter};

impl Config {
    pub fn init_logging(&self) {
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(&self.logging.level));

        match self.logging.format.as_str() {
            "json" => {
                fmt()
                    .json()
                    .with_env_filter(filter)
                    .init();
            }
            _ => {
                fmt()
                    .with_env_filter(filter)
                    .init();
            }
        }
    }
}
```

**Step 2: Update src/main.rs to use logging**

```rust
mod config;
mod error;

use config::Config;
use tracing::{info, error};

fn main() {
    let _ = dotenvy::dotenv();

    let config = match Config::load("config.toml") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    };

    config.init_logging();

    info!(ws_url = %config.network.ws_url, "edgelord starting");
}
```

**Step 3: Verify logging works**

Run: `cargo run`
Expected: Shows timestamped log output with "edgelord starting"

Run: `RUST_LOG=debug cargo run`
Expected: Shows debug-level output

**Step 4: Commit**

```bash
git add src/config.rs src/main.rs
git commit -m "feat(logging): add structured logging with tracing"
```

---

## Task 5: Create WebSocket Message Types

**Files:**
- Create: `src/websocket/mod.rs`
- Create: `src/websocket/messages.rs`
- Modify: `src/main.rs`

**Step 1: Create directory structure**

Run: `mkdir -p src/websocket`

**Step 2: Create src/websocket/messages.rs**

```rust
use rust_decimal::Decimal;
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
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
    pub timestamp: Option<String>,
    pub hash: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PriceChangeMessage {
    pub asset_id: String,
    pub market: Option<String>,
    pub price: Option<String>,
    pub changes: Option<Vec<PriceLevel>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PriceLevel {
    pub price: String,
    pub size: String,
}

impl PriceLevel {
    pub fn price_decimal(&self) -> Option<Decimal> {
        self.price.parse().ok()
    }

    pub fn size_decimal(&self) -> Option<Decimal> {
        self.size.parse().ok()
    }
}
```

**Step 3: Create src/websocket/mod.rs**

```rust
mod messages;

pub use messages::*;
```

**Step 4: Update src/main.rs to declare module**

```rust
mod config;
mod error;
mod websocket;

use config::Config;
use tracing::info;

fn main() {
    let _ = dotenvy::dotenv();

    let config = match Config::load("config.toml") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    };

    config.init_logging();

    info!(ws_url = %config.network.ws_url, "edgelord starting");
}
```

**Step 5: Verify it compiles**

Run: `cargo build`
Expected: Compiles (may have unused warnings, that's ok)

**Step 6: Commit**

```bash
git add src/websocket/
git commit -m "feat(websocket): add message types for Polymarket protocol"
```

---

## Task 6: Create WebSocket Handler

**Files:**
- Create: `src/websocket/handler.rs`
- Modify: `src/websocket/mod.rs`

**Step 1: Create src/websocket/handler.rs**

```rust
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async,
    tungstenite::Message,
    MaybeTlsStream,
    WebSocketStream,
};
use tracing::{debug, error, info, warn};

use crate::error::Result;
use super::messages::{SubscribeMessage, WsMessage};

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

    pub async fn run<F>(
        &self,
        asset_ids: Vec<String>,
        mut on_message: F,
    ) -> Result<()>
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

**Step 2: Update src/websocket/mod.rs**

```rust
mod handler;
mod messages;

pub use handler::WebSocketHandler;
pub use messages::*;
```

**Step 3: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/websocket/handler.rs src/websocket/mod.rs
git commit -m "feat(websocket): add WebSocket connection handler"
```

---

## Task 7: Fetch Active Markets from REST API

**Files:**
- Create: `src/api/mod.rs`
- Create: `src/api/client.rs`
- Create: `src/api/types.rs`
- Modify: `src/main.rs`

**Step 1: Create directory**

Run: `mkdir -p src/api`

**Step 2: Create src/api/types.rs**

```rust
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

**Step 3: Create src/api/client.rs**

```rust
use reqwest::Client;
use tracing::{debug, info};

use crate::error::Result;
use super::types::{Market, MarketsResponse};

pub struct ApiClient {
    client: Client,
    base_url: String,
}

impl ApiClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    /// Fetch active markets, limited to a reasonable number for initial testing
    pub async fn get_active_markets(&self, limit: usize) -> Result<Vec<Market>> {
        let url = format!("{}/markets?active=true&closed=false&limit={}", self.base_url, limit);

        info!(url = %url, "Fetching active markets");

        let response: MarketsResponse = self.client
            .get(&url)
            .send()
            .await?
            .json()
            .await?;

        let markets = response.data.unwrap_or_default();
        debug!(count = markets.len(), "Fetched markets");

        Ok(markets)
    }
}
```

**Step 4: Create src/api/mod.rs**

```rust
mod client;
mod types;

pub use client::ApiClient;
pub use types::*;
```

**Step 5: Update src/main.rs to declare module**

```rust
mod api;
mod config;
mod error;
mod websocket;

use config::Config;
use tracing::info;

fn main() {
    let _ = dotenvy::dotenv();

    let config = match Config::load("config.toml") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    };

    config.init_logging();

    info!(ws_url = %config.network.ws_url, "edgelord starting");
}
```

**Step 6: Verify it compiles**

Run: `cargo build`
Expected: Compiles (with unused warnings)

**Step 7: Commit**

```bash
git add src/api/
git commit -m "feat(api): add REST client for fetching markets"
```

---

## Task 8: Wire Everything Together in Main

**Files:**
- Modify: `src/main.rs`

**Step 1: Update src/main.rs with async main**

```rust
mod api;
mod config;
mod error;
mod websocket;

use api::ApiClient;
use config::Config;
use tracing::{error, info};
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

    if let Err(e) = run(config).await {
        error!(error = %e, "Fatal error");
        std::process::exit(1);
    }
}

async fn run(config: Config) -> error::Result<()> {
    // Fetch some active markets
    let api = ApiClient::new(config.network.api_url.clone());
    let markets = api.get_active_markets(5).await?;

    if markets.is_empty() {
        info!("No active markets found");
        return Ok(());
    }

    // Collect token IDs to subscribe to
    let token_ids: Vec<String> = markets
        .iter()
        .flat_map(|m| m.token_ids())
        .collect();

    info!(
        markets = markets.len(),
        tokens = token_ids.len(),
        "Subscribing to markets"
    );

    for market in &markets {
        info!(
            condition_id = %market.condition_id,
            question = ?market.question,
            tokens = market.tokens.len(),
            "Market"
        );
    }

    // Connect to WebSocket and listen
    let handler = WebSocketHandler::new(config.network.ws_url);

    handler.run(token_ids, |msg| {
        match msg {
            WsMessage::Book(book) => {
                info!(
                    asset_id = %book.asset_id,
                    bids = book.bids.len(),
                    asks = book.asks.len(),
                    "Order book snapshot"
                );

                if let Some(best_bid) = book.bids.first() {
                    info!(
                        asset_id = %book.asset_id,
                        price = %best_bid.price,
                        size = %best_bid.size,
                        "Best bid"
                    );
                }
                if let Some(best_ask) = book.asks.first() {
                    info!(
                        asset_id = %book.asset_id,
                        price = %best_ask.price,
                        size = %best_ask.size,
                        "Best ask"
                    );
                }
            }
            WsMessage::PriceChange(change) => {
                info!(
                    asset_id = %change.asset_id,
                    price = ?change.price,
                    "Price change"
                );
            }
            WsMessage::TickSizeChange(_) => {
                info!("Tick size change");
            }
            WsMessage::Unknown => {
                // Ignore unknown messages
            }
        }
    }).await?;

    Ok(())
}
```

**Step 2: Run and verify live data**

Run: `cargo run`
Expected:
- Fetches markets from API
- Connects to WebSocket
- Prints order book snapshots and price changes

Let it run for 10-20 seconds to see updates flow in.

**Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire up WebSocket to display live market data"
```

---

## Task 9: Add Graceful Shutdown

**Files:**
- Modify: `src/main.rs`

**Step 1: Update main.rs to handle Ctrl+C**

```rust
mod api;
mod config;
mod error;
mod websocket;

use api::ApiClient;
use config::Config;
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
    let api = ApiClient::new(config.network.api_url.clone());
    let markets = api.get_active_markets(5).await?;

    if markets.is_empty() {
        warn!("No active markets found");
        return Ok(());
    }

    let token_ids: Vec<String> = markets
        .iter()
        .flat_map(|m| m.token_ids())
        .collect();

    info!(
        markets = markets.len(),
        tokens = token_ids.len(),
        "Subscribing to markets"
    );

    for market in &markets {
        info!(
            condition_id = %market.condition_id,
            question = ?market.question,
            "Market"
        );
    }

    let handler = WebSocketHandler::new(config.network.ws_url);

    handler.run(token_ids, |msg| {
        match msg {
            WsMessage::Book(book) => {
                let best_bid = book.bids.first().map(|b| b.price.as_str()).unwrap_or("-");
                let best_ask = book.asks.first().map(|a| a.price.as_str()).unwrap_or("-");

                info!(
                    asset = %book.asset_id,
                    bid = %best_bid,
                    ask = %best_ask,
                    "Book"
                );
            }
            WsMessage::PriceChange(change) => {
                info!(
                    asset = %change.asset_id,
                    price = ?change.price,
                    "Price"
                );
            }
            _ => {}
        }
    }).await?;

    Ok(())
}
```

**Step 2: Test graceful shutdown**

Run: `cargo run`
Then press Ctrl+C
Expected: Logs "Shutdown signal received" and "edgelord stopped"

**Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: add graceful shutdown on Ctrl+C"
```

---

## Task 10: Final Verification and Cleanup

**Step 1: Run clippy for lints**

Run: `cargo clippy`
Expected: No errors (warnings are ok for now)

**Step 2: Format code**

Run: `cargo fmt`
Expected: Code formatted

**Step 3: Run full test**

Run: `cargo run`
Expected:
- Connects to Polymarket
- Shows live bid/ask data
- Ctrl+C cleanly shuts down

**Step 4: Commit any formatting changes**

```bash
git add -A
git commit -m "chore: format code" --allow-empty
```

**Step 5: Push the branch**

```bash
git push -u origin feat/phase-1-foundation
```

---

## Summary

**Phase 1 Complete.** You now have:

- Cargo project with all dependencies
- Error types using thiserror
- TOML configuration loading
- Structured logging with tracing
- WebSocket message types for Polymarket protocol
- WebSocket connection handler with reconnect-ready structure
- REST API client for fetching markets
- Live market data streaming to terminal
- Graceful shutdown handling

**Milestone Achieved:** Terminal shows live price updates from Polymarket.

**Next Phase:** Phase 2 will add the OrderBook cache and arbitrage detection.

---

## Files Created

```
edgelord/
├── Cargo.toml
├── config.toml
├── .env.example
├── .gitignore
└── src/
    ├── main.rs
    ├── config.rs
    ├── error.rs
    ├── api/
    │   ├── mod.rs
    │   ├── client.rs
    │   └── types.rs
    └── websocket/
        ├── mod.rs
        ├── handler.rs
        └── messages.rs
```
