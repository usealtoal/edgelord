# edgelord System Architecture

> "Finding edges like a true edgelord"

## Design Philosophy

1. **Hot path must be <40ms** — Compete with sophisticated actors
2. **Domain-driven design** — Exchange-agnostic core, exchange-specific adapters
3. **Simple arbitrage first** — 99.7% of profits came from simple cases
4. **Fail safe** — Never lose money on a "guaranteed" trade
5. **Proper encapsulation** — Private fields, builder patterns, type safety

---

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     RUST CORE (tokio async)                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │
│  │  WebSocket   │───▶│   Detector   │───▶│   Executor   │      │
│  │   Handler    │    │   (domain)   │    │   (traits)   │      │
│  └──────────────┘    └──────────────┘    └──────────────┘      │
│         │                   │                    │              │
│         ▼                   ▼                    ▼              │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │
│  │  OrderBook   │    │  Opportunity │    │  Polymarket  │      │
│  │    Cache     │    │   Builder    │    │   Executor   │      │
│  │  (RwLock)    │    │              │    │              │      │
│  └──────────────┘    └──────────────┘    └──────────────┘      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Module Structure

```
src/
├── lib.rs                 # Library root with public API
├── main.rs                # Thin entry point
├── app.rs                 # Application orchestration
│
├── domain/                # Exchange-agnostic (NO exchange imports)
│   ├── ids.rs             # TokenId, MarketId (newtypes)
│   ├── money.rs           # Price, Volume types
│   ├── market.rs          # MarketPair, MarketInfo
│   ├── orderbook.rs       # OrderBook, OrderBookCache
│   ├── opportunity.rs     # Opportunity with builder
│   ├── position.rs        # Position tracking
│   └── detector.rs        # Detection logic
│
├── exchange/              # Abstraction layer
│   └── traits.rs          # ExchangeClient, OrderExecutor
│
└── polymarket/            # Polymarket implementation
    ├── client.rs          # REST client
    ├── executor.rs        # OrderExecutor implementation
    ├── websocket.rs       # WS handler
    ├── messages.rs        # WS types + to_orderbook()
    ├── registry.rs        # YES/NO pair mapping
    └── types.rs           # API types
```

---

## Component Breakdown

### 1. WebSocket Handler
**Responsibility:** Maintain real-time connection to Polymarket CLOB

```rust
// Pseudo-structure
struct WebSocketHandler {
    connection: WebSocketStream,
    subscribed_markets: HashSet<TokenId>,
    order_book_tx: Sender<OrderBookUpdate>,
}
```

**Key requirements:**
- Auto-reconnect on disconnect
- Subscribe to all active markets
- Parse updates with zero-copy where possible

### 2. Order Book Cache
**Responsibility:** Maintain current state of all order books

```rust
struct OrderBookCache {
    books: DashMap<TokenId, OrderBook>,  // Lock-free concurrent map
}

struct OrderBook {
    bids: BTreeMap<Price, Volume>,  // Sorted by price
    asks: BTreeMap<Price, Volume>,
    last_update: Instant,
}
```

**Key requirements:**
- Lock-free reads (hot path can't wait)
- Atomic updates from WebSocket
- VWAP calculation methods

### 3. Detector Engine
**Responsibility:** Find arbitrage opportunities

```rust
enum ArbitrageOpportunity {
    SingleCondition {
        market_id: MarketId,
        yes_price: Decimal,
        no_price: Decimal,
        profit_per_share: Decimal,
        available_volume: Volume,
    },
    MarketRebalancing {
        market_id: MarketId,
        positions: Vec<(TokenId, Decimal)>,
        total_cost: Decimal,
        profit: Decimal,
    },
    Combinatorial {
        markets: Vec<MarketId>,
        positions: Vec<Position>,
        guaranteed_profit: Decimal,
    },
}
```

**Detection priority:**
1. Simple YES+NO (fastest, most common)
2. Market rebalancing (fast, common)
3. Combinatorial (slow, rare but valuable)

### 4. Executor Engine
**Responsibility:** Execute trades atomically

```rust
struct ExecutorEngine {
    clob_client: ClobClient,
    wallet: LocalWallet,
    pending_orders: Vec<Order>,
}

impl ExecutorEngine {
    async fn execute(&self, opp: ArbitrageOpportunity) -> Result<Execution> {
        // 1. Validate opportunity still exists
        // 2. Calculate position sizes (VWAP-aware)
        // 3. Submit all orders in parallel
        // 4. Monitor fills
        // 5. Handle partial fills
    }
}
```

**Key requirements:**
- All legs submitted within same block (~2s window)
- Slippage protection
- Partial fill handling

### 5. Dependency Graph
**Responsibility:** Track logical relationships between markets

```rust
struct DependencyGraph {
    // Market A implies Market B
    implications: HashMap<MarketId, Vec<MarketId>>,
    // Constraint sets for IP solver
    constraints: Vec<Constraint>,
}
```

**Built offline via:**
- LLM analysis of market descriptions (DeepSeek-R1)
- Manual curation for high-value markets
- Periodic refresh

### 6. Optimization Service
**Responsibility:** Solve combinatorial arbitrage via Frank-Wolfe

```rust
struct OptimizationService {
    gurobi_env: grb::Env,
    projection_cache: HashMap<MarketState, Projection>,
}

impl OptimizationService {
    fn compute_projection(&self, state: &MarketState) -> Projection {
        // Frank-Wolfe algorithm with Gurobi IP oracle
    }
}
```

**Runs in background, results cached for hot path queries.**

---

## Data Flow

```
Block N published on Polygon
         │
         ▼
WebSocket receives update (~5ms)
         │
         ▼
Order Book Cache updated (~1ms)
         │
         ▼
Detector scans for opportunities (~5ms)
         │
    ┌────┴────┐
    │         │
Simple?    Complex?
    │         │
    ▼         ▼
Execute    Query cache
 (~25ms)      │
              ▼
         Cache hit? ──Yes──▶ Execute
              │
              No
              │
              ▼
         Queue for background
         optimization
```

---

## Risk Management

### Position Limits
```rust
struct RiskLimits {
    max_position_per_market: Decimal,    // e.g., $1000
    max_total_exposure: Decimal,          // e.g., $10000
    min_profit_threshold: Decimal,        // e.g., $0.05
    max_slippage_tolerance: Decimal,      // e.g., 2%
}
```

### Circuit Breakers
- Stop trading if drawdown exceeds X%
- Stop trading if execution failure rate spikes
- Stop trading if WebSocket disconnects

### Logging
Every trade logged with:
- Opportunity detected (prices, volumes)
- Decision made (position sizes)
- Execution result (fills, slippage)
- P&L attribution

---

## Tech Stack

```toml
[features]
default = ["polymarket"]
polymarket = ["dep:polymarket-client-sdk", "dep:alloy-signer-local"]

[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = { version = "0.26", features = ["native-tls"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Decimal math (never floats)
rust_decimal = { version = "1", features = ["serde"] }

# Concurrency
parking_lot = "0.12"

# HTTP
reqwest = { version = "0.12", features = ["json"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }

# Polymarket (optional)
polymarket-client-sdk = { version = "0.4", features = ["clob"], optional = true }
alloy-signer-local = { version = "1", optional = true }
```

---

## Development Phases

### Phase 1: Foundation ✅ COMPLETE
- [x] Rust project setup with proper module structure
- [x] WebSocket connection to Polymarket
- [x] Order book cache with thread-safe access
- [x] Configuration and logging

### Phase 2: Detection ✅ COMPLETE
- [x] Single-condition detector (YES + NO < $1)
- [x] Domain types with proper encapsulation
- [x] Opportunity builder pattern
- [x] Comprehensive test coverage

### Phase 3: Execution ✅ COMPLETE
- [x] Exchange trait abstractions (OrderExecutor)
- [x] Polymarket executor implementation
- [x] Position tracking
- [x] Testnet integration (Amoy)

### Phase 4: Risk & Telegram (Next)
- [ ] Risk manager with limits and circuit breakers
- [ ] Telegram bot for alerts and control
- [ ] Daily summary reports

### Phase 5: Mainnet
- [ ] Switch config to mainnet
- [ ] Start with small stakes ($50-100)
- [ ] Monitor and tune thresholds

### Phase 6: Hardening
- [ ] Market rebalancing detector
- [ ] Improved reconnection logic
- [ ] VPS deployment with systemd
