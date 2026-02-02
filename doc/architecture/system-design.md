# edgelord System Architecture

> "Finding edges like a true edgelord"

## Design Philosophy

1. **Hot path must be <40ms** — Compete with sophisticated actors
2. **Pre-compute everything possible** — Heavy math happens offline
3. **Simple arbitrage first** — 95% of profits came from simple cases
4. **Fail safe** — Never lose money on a "guaranteed" trade

---

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     RUST CORE (tokio async)                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │
│  │  WebSocket   │───▶│   Detector   │───▶│   Executor   │      │
│  │   Handler    │    │   Engine     │    │   Engine     │      │
│  └──────────────┘    └──────────────┘    └──────────────┘      │
│         │                   │                    │              │
│         ▼                   ▼                    ▼              │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │
│  │  Order Book  │    │  Dependency  │    │   Parallel   │      │
│  │    Cache     │    │    Graph     │    │  RPC Submit  │      │
│  │  (lockfree)  │    │  (prebuilt)  │    │              │      │
│  └──────────────┘    └──────────────┘    └──────────────┘      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼ (async, non-blocking)
┌─────────────────────────────────────────────────────────────────┐
│              OPTIMIZATION SERVICE (background)                  │
├─────────────────────────────────────────────────────────────────┤
│  Gurobi C API ← Rust FFI bindings (grb crate)                  │
│  Frank-Wolfe solver for combinatorial arbitrage                 │
│  Pre-computes projections, Rust core queries cache              │
└─────────────────────────────────────────────────────────────────┘
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
[dependencies]
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = "0.21"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
crossbeam = "0.8"
dashmap = "5"
rust_decimal = "1"
grb = "2"
ethers = "2"
reqwest = { version = "0.11", features = ["json"] }
tracing = "0.1"
tracing-subscriber = "0.3"

[dependencies.rs-clob-client]
git = "https://github.com/Polymarket/rs-clob-client"
```

---

## Development Phases

### Phase 1: Infrastructure (Week 1-2)
- [ ] Rust project setup
- [ ] WebSocket connection to Polymarket
- [ ] Order book cache structure
- [ ] Basic logging/monitoring

### Phase 2: Simple Arbitrage (Week 3-4)
- [ ] YES+NO detector
- [ ] Market rebalancing detector
- [ ] Execution engine (paper trading)
- [ ] Testnet deployment (Amoy)

### Phase 3: Production Hardening (Week 5-6)
- [ ] Risk management
- [ ] Slippage protection
- [ ] Monitoring dashboard
- [ ] Mainnet deployment (small capital)

### Phase 4: Combinatorial (Week 7-8)
- [ ] Dependency graph builder
- [ ] Gurobi integration
- [ ] Frank-Wolfe implementation
- [ ] Backtest framework

### Phase 5: Scale (Week 9+)
- [ ] Latency optimization
- [ ] Capital scaling
- [ ] Additional markets
