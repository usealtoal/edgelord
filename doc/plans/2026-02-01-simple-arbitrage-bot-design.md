# Simple Arbitrage Bot Design

> Design document for edgelord v1 — a Polymarket arbitrage bot focused on simple, high-volume opportunities.

## Scope

**In scope:**
- Single-condition arbitrage (YES + NO < $1.00)
- Market rebalancing arbitrage (all outcomes < $1.00)
- Telegram alerts and control
- Risk management with circuit breakers

**Out of scope (intentionally):**
- Combinatorial arbitrage (0.24% of profits, 10x complexity)
- Frank-Wolfe / Gurobi optimization
- Dependency graph analysis

**Rationale:** Research shows $39.6M from simple arbitrage vs $95K from combinatorial. We capture 99.7% of profit potential with 30% of the complexity.

---

## Constraints

| Constraint | Value | Rationale |
|------------|-------|-----------|
| Starting capital | $5,000 | User-defined |
| Language | Rust | Maximum latency edge, learning investment |
| Deployment | Cloud VPS (US East) | Reliability, ~20-30ms to RPC |
| Min edge | $0.05 | Smaller edges eaten by execution risk |
| Min profit | $0.50 | Not worth the risk below this |

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    RUST CORE (tokio)                    │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────┐  │
│  │  WebSocket   │───▶│   Detector   │───▶│ Executor │  │
│  │   Handler    │    │              │    │          │  │
│  └──────────────┘    └──────────────┘    └──────────┘  │
│         │                   │                  │        │
│         ▼                   │                  ▼        │
│  ┌──────────────┐           │           ┌──────────┐   │
│  │  OrderBook   │           │           │ Telegram │   │
│  │    Cache     │           │           │   Bot    │   │
│  └──────────────┘           │           └──────────┘   │
│                             ▼                          │
│                    ┌──────────────┐                    │
│                    │     Risk     │                    │
│                    │   Manager    │                    │
│                    └──────────────┘                    │
└─────────────────────────────────────────────────────────┘
```

---

## Core Types

Types designed with proper encapsulation and builder patterns:

```rust
// Identifiers — newtypes with private fields and accessors
pub struct TokenId(String);
impl TokenId {
    pub fn new(id: impl Into<String>) -> Self;
    pub fn as_str(&self) -> &str;
}

pub struct MarketId(String);
impl MarketId {
    pub fn new(id: impl Into<String>) -> Self;
    pub fn as_str(&self) -> &str;
}

// Money — always use Decimal, never f64
pub type Price = rust_decimal::Decimal;
pub type Volume = rust_decimal::Decimal;

// Market pair (binary YES/NO)
pub struct MarketPair {
    market_id: MarketId,
    question: String,
    yes_token: TokenId,
    no_token: TokenId,
}
impl MarketPair {
    pub fn new(...) -> Self;
    pub fn market_id(&self) -> &MarketId;
    pub fn question(&self) -> &str;
    pub fn yes_token(&self) -> &TokenId;
    pub fn no_token(&self) -> &TokenId;
}

// Order book with private fields
pub struct PriceLevel { price: Price, size: Volume }
impl PriceLevel {
    pub fn new(price: Price, size: Volume) -> Self;
    pub fn price(&self) -> Price;
    pub fn size(&self) -> Volume;
}

pub struct OrderBook { token_id: TokenId, bids: Vec<PriceLevel>, asks: Vec<PriceLevel> }
impl OrderBook {
    pub fn new(token_id: TokenId) -> Self;
    pub fn with_levels(token_id: TokenId, bids: Vec<PriceLevel>, asks: Vec<PriceLevel>) -> Self;
    pub fn best_bid(&self) -> Option<&PriceLevel>;
    pub fn best_ask(&self) -> Option<&PriceLevel>;
}

// Opportunities — builder pattern with automatic field calculation
let opportunity = Opportunity::builder()
    .market_id(id)
    .question("Will X happen?")
    .yes_token(yes_token, yes_price)
    .no_token(no_token, no_price)
    .volume(volume)
    .build()?;  // Calculates total_cost, edge, expected_profit

impl Opportunity {
    pub fn market_id(&self) -> &MarketId;
    pub fn yes_ask(&self) -> Price;
    pub fn no_ask(&self) -> Price;
    pub fn total_cost(&self) -> Price;
    pub fn edge(&self) -> Price;
    pub fn volume(&self) -> Volume;
    pub fn expected_profit(&self) -> Price;
}

// Positions — track what we hold
pub struct Position { id: PositionId, market_id: MarketId, legs: Vec<PositionLeg>, ... }
impl Position {
    pub fn new(id, market_id, legs, guaranteed_payout) -> Self;  // Calculates entry_cost
    pub fn id(&self) -> PositionId;
    pub fn entry_cost(&self) -> Price;
    pub fn expected_profit(&self) -> Price;
    pub fn is_open(&self) -> bool;
    pub fn close(&mut self, pnl: Price);
}

pub enum PositionStatus {
    Open,
    PartialFill { filled: Vec<TokenId>, missing: Vec<TokenId> },
    Closed { pnl: Price, closed_at: DateTime<Utc> },
}

// Execution results (in exchange/traits.rs)
pub enum ExecutionResult {
    Success { yes_order: OrderId, no_order: OrderId, position: Position },
    PartialFill { filled_order: OrderId, filled_leg: TokenId, failed_leg: TokenId, error: String },
    Failed { reason: String },
}

// Structured errors (in error.rs)
pub enum ConfigError {
    MissingField { field: &'static str },
    InvalidValue { field: &'static str, message: String },
    ReadFile(std::io::Error),
    Parse(toml::de::Error),
}

pub enum ExecutionError {
    AuthFailed { reason: String },
    InvalidTokenId { token_id: String, reason: String },
    OrderRejected { reason: String },
    OrderBuildFailed { reason: String },
    SigningFailed { reason: String },
    SubmissionFailed { reason: String },
    PartialFill { filled_leg: TokenId, failed_leg: TokenId, error: String },
}
```

---

## Detection Logic

### Single-Condition Arbitrage

```rust
pub fn detect_single(
    market: &Market,
    yes_book: &OrderBook,
    no_book: &OrderBook,
    config: &DetectorConfig,
) -> Option<Opportunity> {
    let yes_ask = yes_book.best_ask()?;
    let no_ask = no_book.best_ask()?;

    let total_cost = yes_ask.price + no_ask.price;
    let edge = Price::ONE - total_cost;

    if edge < config.min_edge {
        return None;
    }

    let volume = yes_ask.volume.min(no_ask.volume);
    let expected_profit = edge * volume;

    if expected_profit < config.min_profit {
        return None;
    }

    Some(Opportunity::SingleCondition { /* ... */ })
}
```

### Market Rebalancing

```rust
pub fn detect_rebalancing(
    market: &Market,
    books: &HashMap<TokenId, OrderBook>,
    config: &DetectorConfig,
) -> Option<Opportunity> {
    if market.outcomes.len() < 3 {
        return None;  // Single-condition handles 2-outcome markets
    }

    let legs: Vec<RebalanceLeg> = market.outcomes.iter()
        .filter_map(|outcome| {
            let book = books.get(&outcome.token_id)?;
            let ask = book.best_ask()?;
            Some(RebalanceLeg {
                token_id: outcome.token_id.clone(),
                ask_price: ask.price,
            })
        })
        .collect();

    if legs.len() != market.outcomes.len() {
        return None;  // Missing orderbook data
    }

    let total_cost: Price = legs.iter().map(|l| l.ask_price).sum();
    let edge = Price::ONE - total_cost;

    if edge < config.min_edge {
        return None;
    }

    let volume = legs.iter()
        .filter_map(|l| books.get(&l.token_id)?.best_ask())
        .map(|ask| ask.volume)
        .min()?;

    let expected_profit = edge * volume;

    if expected_profit < config.min_profit {
        return None;
    }

    Some(Opportunity::Rebalancing { /* ... */ })
}
```

---

## Execution Logic

### Flow

```
1. VALIDATE (< 5ms)
   - Re-check prices haven't moved beyond tolerance
   - Verify under position limits
   - Confirm opportunity exceeds min_edge

2. SIZE (< 1ms)
   - volume = min(available across legs, max_position / total_cost)
   - Apply VWAP if eating multiple levels

3. SUBMIT (< 30ms)
   - Build all orders
   - Submit ALL legs in parallel via tokio::join!
   - Target: all orders in same ~2s block window

4. MONITOR (async)
   - Track fill status via WebSocket user channel
   - Log partial fills
   - Alert on failures
```

### Order Building

```rust
pub struct OrderBuilder {
    client: ClobClient,
}

impl OrderBuilder {
    pub fn build_order(
        &self,
        token_id: &TokenId,
        side: Side,
        price: Price,
        volume: Volume,
    ) -> Order {
        Order {
            token_id: token_id.clone(),
            side,
            price,
            size: volume,
            order_type: OrderType::Limit,  // Or FOK if supported
            time_in_force: TimeInForce::GTC,
        }
    }

    pub async fn submit_parallel(&self, orders: Vec<Order>) -> Vec<OrderResult> {
        let futures: Vec<_> = orders.into_iter()
            .map(|order| self.client.submit(order))
            .collect();

        futures::future::join_all(futures).await
    }
}
```

### Partial Fill Recovery

```rust
pub async fn handle_partial_fill(
    executor: &Executor,
    position: &Position,
) -> Result<(), ExecutionError> {
    // Identify exposed leg
    let PositionStatus::PartialFill { filled, missing } = &position.status else {
        return Ok(());
    };

    // Attempt to close the filled positions at market
    for token_id in filled {
        let book = executor.get_book(token_id).await?;
        let best_bid = book.best_bid().ok_or(ExecutionError::InsufficientLiquidity)?;

        executor.submit_market_sell(token_id, position.leg_volume(token_id)).await?;
    }

    // Alert regardless of outcome
    executor.alert(Alert::PartialFillRecovery { position_id: position.id }).await;

    Ok(())
}
```

---

## Risk Management

### Configuration

```rust
pub struct RiskConfig {
    // Position limits
    pub max_position_size: Price,      // $500 (10% of capital)
    pub max_total_exposure: Price,     // $2,500 (50% of capital)

    // Loss limits
    pub max_daily_loss: Price,         // $250 (5% of capital)

    // Opportunity thresholds
    pub min_edge: Price,               // $0.05
    pub min_profit: Price,             // $0.50

    // Circuit breakers
    pub max_consecutive_failures: u32, // 3
    pub ws_disconnect_tolerance: Duration,  // 10 seconds
    pub max_execution_latency: Duration,    // 500ms

    // Safety margins
    pub settlement_buffer: Duration,   // 1 hour before market ends
}
```

### Risk Manager

```rust
pub struct RiskManager {
    config: RiskConfig,
    state: RwLock<RiskState>,
}

struct RiskState {
    current_exposure: Price,
    daily_pnl: Price,
    consecutive_failures: u32,
    paused: bool,
    pause_reason: Option<String>,
}

impl RiskManager {
    pub fn check(&self, opportunity: &Opportunity) -> Result<(), RiskRejection> {
        let state = self.state.read();

        if state.paused {
            return Err(RiskRejection::CircuitBreakerActive);
        }

        let required_capital = opportunity.total_cost();

        if state.current_exposure + required_capital > self.config.max_total_exposure {
            return Err(RiskRejection::ExposureLimitExceeded);
        }

        if required_capital > self.config.max_position_size {
            return Err(RiskRejection::ExposureLimitExceeded);
        }

        // Ensure we can survive this trade going to zero
        let potential_loss = required_capital;
        if state.daily_pnl - potential_loss < -self.config.max_daily_loss {
            return Err(RiskRejection::DailyLossLimitExceeded);
        }

        Ok(())
    }

    pub fn record_execution(&self, result: &ExecutionResult) {
        let mut state = self.state.write();

        match result {
            ExecutionResult::Success { position } => {
                state.current_exposure += position.entry_cost;
                state.consecutive_failures = 0;
            }
            ExecutionResult::Failed { .. } => {
                state.consecutive_failures += 1;
                if state.consecutive_failures >= self.config.max_consecutive_failures {
                    state.paused = true;
                    state.pause_reason = Some("Consecutive failures".into());
                }
            }
            ExecutionResult::PartialFill { exposure, .. } => {
                state.current_exposure += exposure;
            }
        }
    }

    pub fn record_close(&self, pnl: Price) {
        let mut state = self.state.write();
        state.daily_pnl += pnl;

        if state.daily_pnl < -self.config.max_daily_loss {
            state.paused = true;
            state.pause_reason = Some("Daily loss limit".into());
        }
    }
}
```

---

## Telegram Interface

### Alerts

| Event | Format |
|-------|--------|
| Trade executed | `✅ {market} @ ${cost} → ${profit} profit` |
| Trade failed | `❌ {market}: {reason}` |
| Partial fill | `⚠️ {market}: partial fill, exposed ${amount}` |
| Circuit breaker | `🛑 Paused: {reason}` |
| Connection issue | `⚠️ WebSocket disconnected` |
| Reconnected | `🟢 Reconnected` |
| Daily summary | `📊 {trades} trades, ${pnl}, {fill_rate}% fill rate` |

### Commands

| Command | Description |
|---------|-------------|
| `/status` | P&L, positions, exposure, uptime |
| `/pause` | Stop new trades |
| `/resume` | Resume trading |
| `/limits` | Show risk limits |
| `/set <limit> <value>` | Adjust limit (requires confirm) |
| `/positions` | List open positions |
| `/kill` | Emergency close all (requires confirm) |

### Implementation

```rust
pub struct TelegramBot {
    bot: teloxide::Bot,
    allowed_user: UserId,
    alert_tx: mpsc::Sender<Alert>,
    command_rx: mpsc::Receiver<Command>,
}

impl TelegramBot {
    pub async fn run(self, app_state: Arc<AppState>) {
        let handler = dptree::entry()
            .filter(|msg: Message| msg.from().map(|u| u.id) == Some(self.allowed_user))
            .branch(
                dptree::entry()
                    .filter_command::<BotCommand>()
                    .endpoint(handle_command)
            );

        Dispatcher::builder(self.bot, handler)
            .dependencies(dptree::deps![app_state])
            .build()
            .dispatch()
            .await;
    }
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum BotCommand {
    Status,
    Pause,
    Resume,
    Limits,
    #[command(parse_with = "split")]
    Set { limit: String, value: String },
    Positions,
    Kill,
}
```

---

## Configuration

### File: `config.toml`

```toml
[network]
ws_url = "wss://ws-subscriptions-clob.polymarket.com/ws/"
api_url = "https://clob.polymarket.com"
chain = "polygon"  # or "amoy" for testnet

[risk]
max_position_size = 500.0
max_total_exposure = 2500.0
max_daily_loss = 250.0
min_edge = 0.05
min_profit = 0.50
max_consecutive_failures = 3
ws_disconnect_tolerance_secs = 10
max_execution_latency_ms = 500
settlement_buffer_hours = 1

[telegram]
enabled = true
# bot_token loaded from TELEGRAM_BOT_TOKEN env var
# allowed_user loaded from TELEGRAM_USER_ID env var

[logging]
level = "info"  # debug, info, warn, error
format = "json"  # json or pretty
```

### File: `.env.example`

```bash
# Polymarket
POLYMARKET_API_KEY=
POLYMARKET_SECRET=
POLYMARKET_PASSPHRASE=

# Wallet
WALLET_PRIVATE_KEY=

# Telegram
TELEGRAM_BOT_TOKEN=
TELEGRAM_USER_ID=

# RPC
POLYGON_RPC_URL=https://polygon-mainnet.g.alchemy.com/v2/YOUR_KEY
```

---

## Error Handling

### Strategy

1. **Recoverable errors** — Retry with backoff, then skip opportunity
2. **Connection errors** — Reconnect automatically, pause trading during gap
3. **Execution errors** — Log, alert, trigger circuit breaker if repeated
4. **Configuration errors** — Fail fast at startup

### Error Types

```rust
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::Error),

    #[error("API error: {0}")]
    Api(#[from] reqwest::Error),

    #[error("Execution failed: {0}")]
    Execution(#[from] ExecutionError),

    #[error("Risk check failed: {0}")]
    Risk(#[from] RiskRejection),
}
```

---

## Testing Strategy

### Unit Tests

- Detection logic with mock orderbooks
- Risk manager state transitions
- Order building correctness
- VWAP calculations

### Integration Tests

- WebSocket connection and parsing (against testnet)
- Order submission (against testnet)
- Telegram bot commands (mock)

### Simulation

- Replay historical orderbook data
- Measure detection latency
- Validate profit calculations

---

## Deployment

### VPS Requirements

- **Provider:** DigitalOcean, Vultr, or AWS Lightsail
- **Region:** US East (NYC or Virginia) — closest to Polymarket infrastructure
- **Specs:** 2 vCPU, 4GB RAM, 50GB SSD
- **Cost:** ~$20-30/month

### Deployment Steps

```bash
# 1. Build release binary
cargo build --release

# 2. Copy to server
scp target/release/edgelord user@server:/opt/edgelord/

# 3. Copy config
scp config.toml user@server:/opt/edgelord/
scp .env user@server:/opt/edgelord/

# 4. Set up systemd service
sudo cp edgelord.service /etc/systemd/system/
sudo systemctl enable edgelord
sudo systemctl start edgelord
```

### Systemd Service: `edgelord.service`

```ini
[Unit]
Description=Edgelord Polymarket Bot
After=network.target

[Service]
Type=simple
User=edgelord
WorkingDirectory=/opt/edgelord
ExecStart=/opt/edgelord/edgelord
Restart=always
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
```

### Monitoring

- **Logs:** `journalctl -u edgelord -f`
- **Health:** Telegram `/status` command
- **Alerts:** Telegram notifications on all events

---

## Project Structure

```
edgelord/
├── Cargo.toml
├── config.toml
├── .env.example
├── CONTRIBUTING.md
│
├── src/
│   ├── lib.rs               # Library root with public API
│   ├── main.rs              # Thin binary entry point
│   ├── app.rs               # Application orchestration
│   ├── config.rs            # Configuration loading
│   ├── error.rs             # Structured error types (ConfigError, ExecutionError)
│   │
│   ├── domain/              # Exchange-agnostic core (no exchange imports!)
│   │   ├── mod.rs           # Public exports
│   │   ├── id.rs           # TokenId, MarketId (newtypes with encapsulation)
│   │   ├── money.rs         # Price, Volume (type aliases + constants)
│   │   ├── market.rs        # MarketPair, MarketInfo, TokenInfo
│   │   ├── orderbook.rs     # PriceLevel, OrderBook, OrderBookCache
│   │   ├── opportunity.rs   # Opportunity with builder pattern
│   │   ├── position.rs      # Position, PositionLeg, PositionTracker
│   │   └── detector.rs      # Detection logic + DetectorConfig
│   │
│   ├── exchange/            # Exchange abstraction layer
│   │   ├── mod.rs
│   │   └── traits.rs        # ExchangeClient, OrderExecutor traits
│   │
│   ├── polymarket/          # Polymarket implementation
│   │   ├── mod.rs           # Re-exports
│   │   ├── client.rs        # REST API client (implements ExchangeClient)
│   │   ├── executor.rs      # Order execution (implements OrderExecutor)
│   │   ├── websocket.rs     # WebSocket connection and handling
│   │   ├── messages.rs      # WS message types + to_orderbook() conversion
│   │   ├── types.rs         # API response types (Market, Token)
│   │   └── registry.rs      # MarketRegistry (YES/NO pair mapping)
│   │
│   ├── risk/                # (Phase 4+)
│   │   ├── mod.rs
│   │   └── manager.rs       # Limits, circuit breakers
│   │
│   └── telegram/            # (Phase 4+)
│       ├── mod.rs
│       ├── bot.rs           # Command handlers
│       └── alerts.rs        # Send notifications
│
├── tests/
│   ├── detection_tests.rs
│   └── integration/
│       └── testnet.rs
│
└── doc/
    ├── research/
    ├── architecture/
    └── plans/
```

### Architecture Principles

- **Domain-driven design:** `domain/` contains exchange-agnostic types and logic
- **Trait-based abstractions:** `exchange/traits.rs` defines `ExchangeClient` and `OrderExecutor`
- **Clean separation:** `polymarket/` implements traits, `domain/` has no exchange imports
- **Proper encapsulation:** Private fields with accessor methods, builder patterns
- **Type safety:** Newtypes for identifiers, Decimal for money (never floats)
- **Feature flags:** `polymarket` feature optional, domain builds standalone

### Dependencies

```toml
[package]
name = "edgelord"
version = "0.1.0"
edition = "2021"

[features]
default = ["polymarket"]
polymarket = ["dep:polymarket-client-sdk", "dep:alloy-signer-local"]

[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }

# WebSocket
tokio-tungstenite = { version = "0.26", features = ["native-tls"] }
futures = "0.3"

# HTTP client
reqwest = { version = "0.12", features = ["json"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Decimal math (never use floats for money)
rust_decimal = { version = "1", features = ["serde"] }
rust_decimal_macros = "1"

# Telegram (Phase 4+)
# teloxide = { version = "0.12", features = ["macros"] }

# Configuration
dotenvy = "0.15"
toml = "0.8"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }

# Error handling
thiserror = "2"

# Time
chrono = { version = "0.4", features = ["serde"] }

# Concurrency
parking_lot = "0.12"

# Async traits
async-trait = "0.1"

# URL parsing
url = "2"

# Polymarket CLOB client (optional, feature-gated)
polymarket-client-sdk = { version = "0.4", features = ["clob"], optional = true }
alloy-signer-local = { version = "1", optional = true }

[dev-dependencies]
tokio-test = "0.4"
```

**Feature flags:**
- `default = ["polymarket"]` — Full functionality with Polymarket support
- `--no-default-features` — Builds domain-only library (for multi-exchange development)

---

## Development Phases

### Phase 1: Foundation ✅ COMPLETE

**Goal:** Connect to Polymarket and see live data.

**Tasks:**
- ✅ Initialize Cargo project with dependencies
- ✅ Implement config loading (`config.rs`)
- ✅ Connect to WebSocket (`polymarket/websocket.rs`)
- ✅ Parse market messages (`polymarket/messages.rs`)
- ✅ Print updates to stdout

**Milestone:** Terminal shows live price updates.

### Phase 2: Detection ✅ COMPLETE

**Goal:** Find arbitrage opportunities in real-time.

**Tasks:**
- ✅ Build OrderBook cache (`domain/orderbook.rs`)
- ✅ Implement single-condition detector (`domain/detector.rs`)
- ✅ Wire detector to WebSocket updates
- ✅ Log opportunities with details

**Milestone:** Logs "ARBITRAGE DETECTED" with edge, volume, expected profit.

### Phase 3: Execution (Testnet) ✅ COMPLETE

**Goal:** Execute trades on Amoy testnet.

**Tasks:**
- ✅ Integrate CLOB API client (`executor/orders.rs`)
- ✅ Implement order building and submission
- ✅ Track positions (`executor/positions.rs`)
- ✅ Wire executor to detection flow

**Milestone:** Execute a trade on testnet.

### Phase 4: Risk & Telegram

**Goal:** Safe operation with remote monitoring.

**Tasks:**
- Implement RiskManager (`risk/manager.rs`)
- Add circuit breakers
- Set up Telegram bot (`telegram/bot.rs`)
- Implement alerts (`telegram/alerts.rs`)

**Milestone:** Receive trade alert on phone.

### Phase 5: Mainnet

**Goal:** Real money, small stakes.

**Tasks:**
- Switch config to mainnet
- Start with $50-100 per trade
- Monitor and tune thresholds
- Fix issues as discovered

**Milestone:** First profitable trade.

### Phase 6: Hardening

**Goal:** Unattended operation.

**Tasks:**
- Add market rebalancing detector (`detector/rebalance.rs`)
- Improve reconnection logic
- Add daily summary reports
- Deploy to VPS
- Set up systemd service

**Milestone:** Running unattended for 24+ hours.

---

## Design Principles

1. **Clarity over cleverness** — Code reads like intent
2. **One module, one job** — Clear boundaries, single responsibility
3. **Types enforce correctness** — Invalid states unrepresentable
4. **Fail safe** — When uncertain, don't trade
5. **Log everything** — Data for tuning later
6. **No premature abstraction** — Three cases before generalizing

---

## Future Considerations

### Kalshi Integration

Kalshi (US-regulated, CFTC-approved) uses a fundamentally different market structure:

- **Single-contract model**: One contract per binary outcome, not separate YES/NO tokens
- **No simple arbitrage**: A YES bid at X¢ = NO ask at (100-X)¢, preventing YES+NO < $1 opportunities
- **Different opportunities**: Multi-outcome events (3+ outcomes) and cross-platform arbitrage

If adding Kalshi support:
1. Detection logic must change (multi-outcome sum < $1, not binary pair)
2. Different API client (kalshi-rust crate, API key auth vs wallet signing)
3. Consider `exchanges/kalshi/` alongside `polymarket/` with shared `domain/` types

---

## References

- [Polymarket CLOB Docs](https://docs.polymarket.com/developers/CLOB/introduction)
- [Arbitrage Research (arXiv:2508.03474)](https://arxiv.org/abs/2508.03474)
- [rs-clob-client](https://github.com/Polymarket/rs-clob-client)
- [teloxide](https://github.com/teloxide/teloxide)
