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

**Components:**
- **WebSocket Handler** — Real-time price feed from Polymarket
- **OrderBook Cache** — Current state of all tracked markets
- **Detector** — Scans for arbitrage opportunities
- **Risk Manager** — Position limits, profit thresholds, circuit breakers
- **Executor** — Submits orders via CLOB API
- **Telegram Bot** — Alerts and commands

---

## Detection Logic

### Single-Condition Arbitrage

```
For each market with YES/NO tokens:
  best_yes_ask = lowest price someone will sell YES
  best_no_ask  = lowest price someone will sell NO

  if best_yes_ask + best_no_ask < 0.95:
    volume = min(yes_volume_at_ask, no_volume_at_ask)
    profit = (1.00 - best_yes_ask - best_no_ask) * volume

    if profit > min_profit_threshold:
      → Emit opportunity
```

### Market Rebalancing

```
For each market with 3+ outcomes:
  total_cost = sum of best_ask for each outcome

  if total_cost < 0.95:
    volume = min(volume across all outcomes)
    profit = (1.00 - total_cost) * volume

    if profit > min_profit_threshold:
      → Emit opportunity
```

**Key insight:** Use best ask, not mid-price. You're taking liquidity — the ask is what you'll actually pay.

---

## Execution Logic

### Flow

```
1. VALIDATE (< 5ms)
   - Re-check prices haven't moved
   - Verify under position limits
   - Confirm opportunity exceeds min_edge

2. SIZE (< 1ms)
   - volume = min(available across legs, max_position)
   - Apply VWAP estimate if eating multiple levels

3. SUBMIT (< 30ms)
   - Build all orders
   - Submit ALL legs in parallel
   - Target: all orders in same ~2s block window

4. MONITOR (async)
   - Track fill status via WebSocket
   - Log partial fills
   - Alert on failures
```

### Critical Rule

All legs or none. If YES fills but NO doesn't, you're exposed.

Mitigations:
- Submit all orders within same block window
- Use FOK (Fill-or-Kill) if supported
- On partial fill, immediately close exposed position

---

## Risk Management

### Limits

```rust
struct RiskLimits {
    max_position_size: Decimal,      // $500 (10% of capital)
    max_total_exposure: Decimal,     // $2,500 (50% of capital)
    max_daily_loss: Decimal,         // $250 (5% of capital)
    min_edge_threshold: Decimal,     // $0.05
    min_profit_threshold: Decimal,   // $0.50
}
```

### Circuit Breakers

| Trigger | Action |
|---------|--------|
| Daily loss > $250 | Pause, alert |
| 3 consecutive failed executions | Pause, alert |
| WebSocket disconnected > 10s | Pause, reconnect |
| Execution latency > 500ms | Skip opportunity |

### Pre-trade Checks

1. Current exposure + new position ≤ max_total_exposure
2. Today's P&L - potential loss > -max_daily_loss
3. Not paused by circuit breaker
4. Market not about to settle

---

## Telegram Interface

### Alerts (bot → user)

| Event | Example |
|-------|---------|
| Trade executed | `✅ Bought YES+NO on "Trump PA" @ $0.94. Profit: $3.00` |
| Trade failed | `❌ Failed to fill NO leg. YES exposed $47. Attempting close.` |
| Circuit breaker | `🛑 Paused: Daily loss limit hit (-$251.30)` |
| Connection issue | `⚠️ WebSocket disconnected. Reconnecting...` |
| Daily summary | `📊 Today: 7 trades, +$34.50, 100% fill rate` |

### Commands (user → bot)

| Command | Action |
|---------|--------|
| `/status` | Current P&L, positions, exposure, uptime |
| `/pause` | Stop new trades |
| `/resume` | Resume trading |
| `/limits` | Show risk limits |
| `/setlimit <name> <value>` | Adjust limit |
| `/positions` | List open positions |
| `/kill` | Emergency close all, pause |

### Security

- Only respond to configured Telegram user ID
- `/kill` and `/setlimit` require confirmation

---

## Data Flow

### Startup

```
1. Load config
2. Initialize OrderBook cache
3. Connect to Polymarket WebSocket
4. Subscribe to markets
5. Start Telegram bot
6. Send "🟢 Bot started" alert
7. Enter main loop
```

### Main Loop

```
WebSocket update → Update cache → Detector.scan()
                                       │
                   ┌───────────────────┴───────────────────┐
                   │                                       │
              No opportunity                          Opportunity
                   │                                       │
                continue                          RiskManager.check()
                                                          │
                                          ┌───────────────┴───────────────┐
                                          │                               │
                                      Rejected                        Approved
                                          │                               │
                                      log, continue                 Executor.execute()
```

### Tokio Tasks

| Task | Purpose |
|------|---------|
| `ws_handler` | WebSocket messages |
| `detector_loop` | Find opportunities |
| `executor` | Execute trades |
| `telegram_bot` | Commands and alerts |
| `health_check` | Connection verification |

---

## Project Structure

```
edgelord/
├── Cargo.toml
├── .env.example
├── config.toml
│
├── src/
│   ├── main.rs
│   ├── config.rs
│   │
│   ├── websocket/
│   │   ├── mod.rs
│   │   ├── handler.rs
│   │   └── messages.rs
│   │
│   ├── orderbook/
│   │   ├── mod.rs
│   │   └── cache.rs
│   │
│   ├── detector/
│   │   ├── mod.rs
│   │   ├── single.rs
│   │   └── rebalance.rs
│   │
│   ├── executor/
│   │   ├── mod.rs
│   │   ├── orders.rs
│   │   └── positions.rs
│   │
│   ├── risk/
│   │   ├── mod.rs
│   │   └── manager.rs
│   │
│   ├── telegram/
│   │   ├── mod.rs
│   │   ├── bot.rs
│   │   └── alerts.rs
│   │
│   └── types.rs
│
└── doc/
```

### Dependencies

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = "0.21"
reqwest = { version = "0.11", features = ["json"] }
rust_decimal = "1"
teloxide = { version = "0.12", features = ["macros"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
dotenvy = "0.15"
toml = "0.8"
```

---

## Development Phases

### Phase 1: Foundation
- Project setup
- Config loading
- WebSocket connection
- Parse and print messages
- **Milestone:** See live price updates

### Phase 2: Detection
- OrderBook cache
- Single-condition detector
- Log opportunities (no execution)
- **Milestone:** Logs "Found $0.06 edge on market X"

### Phase 3: Execution (Testnet)
- CLOB API integration
- Order submission
- Position tracking
- **Milestone:** Execute trade on Amoy testnet

### Phase 4: Risk & Telegram
- Risk Manager
- Circuit breakers
- Telegram bot
- **Milestone:** Receive trade alert on phone

### Phase 5: Mainnet
- Switch to mainnet
- $50-100 trades
- Tune thresholds
- **Milestone:** First real profit

### Phase 6: Hardening
- Market rebalancing detector
- Reconnection logic
- Daily summaries
- VPS deployment
- **Milestone:** Running unattended 24h+

---

## Design Principles

- **Elegance over cleverness** — Clear abstractions, code reads like intent
- **One module, one job** — Each component has a single responsibility
- **Invalid states unrepresentable** — Use types to enforce correctness
- **Fail safe** — When uncertain, don't trade
- **Log everything** — Data for tuning thresholds later

---

## References

- [Polymarket CLOB Docs](https://docs.polymarket.com/developers/CLOB/introduction)
- [Arbitrage Research (arXiv:2508.03474)](https://arxiv.org/abs/2508.03474)
- [rs-clob-client](https://github.com/Polymarket/rs-clob-client)
- [teloxide](https://github.com/teloxide/teloxide)
