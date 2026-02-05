# CLI, Config & Stats Improvements

> Status: Historical
> Superseded by: N/A
> Summary:
> - Goal: Make edgelord feel like a polished, idiomatic Rust CLI with proper observability.
> - Scope: CLI Structure
> Planned Outcomes:
> - CLI Structure
> - Config


**Goal:** Make edgelord feel like a polished, idiomatic Rust CLI with proper observability.

## Current State

### CLI Structure
- ✅ Good subcommand layout (run, status, logs, service, check, wallet)
- ✅ Override flags for common settings
- ❌ No config generation/validation commands
- ❌ No historical stats viewing
- ❌ No DB introspection commands

### Config
- ✅ Well-structured TOML with sections
- ✅ Env vars for secrets
- ❌ No `config init` to generate starter file
- ❌ No way to see "effective" config (with defaults)
- ❌ Validation errors aren't actionable

### Stats
- ❌ Ephemeral (in-memory + JSON dump)
- ❌ Resets daily, no history
- ❌ No per-strategy breakdown
- ❌ Missing key metrics: latency, win rate, avg profit

---

## Proposed Improvements

### Phase 1: Stats Database Tables

Add new tables for persistent metrics:

```sql
-- Individual opportunities (raw data)
CREATE TABLE opportunities (
    id INTEGER PRIMARY KEY,
    strategy TEXT NOT NULL,          -- "single_condition", "combinatorial"
    market_ids TEXT NOT NULL,        -- JSON array
    edge REAL NOT NULL,              -- Expected edge %
    expected_profit REAL NOT NULL,   -- USD
    detected_at TEXT NOT NULL,       -- ISO timestamp
    executed INTEGER NOT NULL,       -- 0 or 1
    execution_id INTEGER             -- FK to trades (if executed)
);

-- Executed trades
CREATE TABLE trades (
    id INTEGER PRIMARY KEY,
    opportunity_id INTEGER NOT NULL,
    strategy TEXT NOT NULL,
    market_ids TEXT NOT NULL,
    entry_prices TEXT NOT NULL,      -- JSON: {"token_id": price}
    exit_prices TEXT,                -- JSON (if closed)
    size REAL NOT NULL,              -- USD
    realized_profit REAL,            -- USD (if closed)
    status TEXT NOT NULL,            -- "open", "closed", "expired"
    opened_at TEXT NOT NULL,
    closed_at TEXT,
    FOREIGN KEY (opportunity_id) REFERENCES opportunities(id)
);

-- Aggregated daily stats (fast queries)
CREATE TABLE daily_stats (
    date TEXT PRIMARY KEY,           -- YYYY-MM-DD
    opportunities_detected INTEGER DEFAULT 0,
    opportunities_executed INTEGER DEFAULT 0,
    trades_opened INTEGER DEFAULT 0,
    trades_closed INTEGER DEFAULT 0,
    profit_realized REAL DEFAULT 0,
    loss_realized REAL DEFAULT 0,
    avg_edge REAL,
    win_count INTEGER DEFAULT 0,
    loss_count INTEGER DEFAULT 0,
    latency_p50_ms INTEGER,
    latency_p95_ms INTEGER,
    peak_exposure REAL DEFAULT 0
);
```

### Phase 2: CLI Commands

```
edgelord config init              # Generate config.toml with comments
edgelord config show              # Show resolved config (with defaults)
edgelord config validate [path]   # Detailed validation

edgelord statistics                    # Today's summary (default)
edgelord statistics today              # Today's detailed breakdown
edgelord statistics week               # Last 7 days
edgelord statistics history [days]     # Historical view
edgelord statistics export [--csv]     # Export for analysis

edgelord db status                # Show DB path, table counts, size
edgelord db migrate               # Run pending migrations
```

### Phase 3: Richer Status Output

```
edgelord status

edgelord v0.1.0
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Status:      ● running (pid 12345)
Uptime:      2d 14h 22m
Exchange:    polymarket (mainnet, chain 137)
Strategies:  single_condition, combinatorial

         Today        Week         All-Time
         ─────────    ─────────    ─────────
Detected 142          823          4,521
Executed 12           67           412
Profit   $24.50       $156.25      $892.40
Win Rate 75%          71%          68%

Positions:   3 open ($450 exposure / $5000 max)
Latency:     p50=8ms  p95=42ms  p99=85ms
```

---

## Implementation Order

1. **DB Schema** - Add new tables via Diesel migration
2. **Stats Recording** - Hook into orchestrator to persist events
3. **Stats Query Layer** - Add `stats::` module for aggregation queries
4. **CLI: stats** - Add stats subcommand group
5. **CLI: config** - Add config subcommand group
6. **CLI: db** - Add db subcommand group
7. **Enhanced Status** - Pull from DB instead of JSON file

---

## Key Metrics to Track

**For Evaluating Success:**
- Total profit (realized)
- Win rate (% of trades profitable)
- Average profit per trade
- Edge accuracy (expected vs actual)
- Opportunity→Execution rate
- Latency percentiles
- Peak vs average exposure
- Per-strategy breakdown

**For Debugging:**
- Rejected opportunities (why: risk, latency, etc.)
- Failed executions (why: slippage, timeout, etc.)
- WebSocket reconnection frequency
- LLM inference latency + cost

---

## Questions

1. **Retention policy?** Keep all raw data, or aggregate after N days?
2. **Real-time vs batch?** Insert per-event, or batch every N seconds?
3. **Status file?** Keep JSON status file, or DB-only?
