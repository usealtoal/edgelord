# Adaptive Subscription Management

**Date:** 2026-02-03
**Status:** Proposed
**Author:** Claude + Ryan

## Overview

Design for a production-grade subscription management system that maximizes arbitrage opportunity coverage while maintaining low latency. The system adapts to available resources, prioritizes high-value markets, and handles connection failures seamlessly.

### Goals

1. **Maximize opportunity coverage** - Subscribe to as many markets as resources allow
2. **Minimize latency** - Stay within processing latency targets
3. **Zero data gaps** - Seamless connection failover with no missed updates
4. **Exchange-agnostic** - Clean abstractions that support multiple exchanges
5. **Self-tuning** - Adapt to load conditions automatically

## Architecture

### High-Level Components

```
┌─────────────────────────────────────────────────────────────┐
│                      Orchestrator                           │
│                                                             │
│  ┌─────────────┐    ┌──────────────┐    ┌───────────────┐  │
│  │ MarketScorer│───▶│ Subscription │───▶│   Adaptive    │  │
│  │   (trait)   │    │   Manager    │    │   Governor    │  │
│  └─────────────┘    └──────────────┘    └───────────────┘  │
│         │                  │                    │          │
│         ▼                  ▼                    ▼          │
│  ┌─────────────┐    ┌──────────────┐    ┌───────────────┐  │
│  │ Polymarket  │    │  Maintains   │    │   Monitors    │  │
│  │ Scorer impl │    │ active subs  │    │   metrics,    │  │
│  │             │    │ + priority   │    │ signals add/  │  │
│  │             │    │   queue      │    │ remove        │  │
│  └─────────────┘    └──────────────┘    └───────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Role |
|-----------|------|
| `MarketScorer` | Exchange-specific: scores markets by arb potential |
| `SubscriptionManager` | Exchange-agnostic: tracks active/pending subs, priority queue |
| `ConnectionPool` | Exchange-agnostic: manages sharded, redundant connections |
| `AdaptiveGovernor` | Exchange-agnostic: monitors performance, decides when to scale |
| `MessageDeduplicator` | Exchange-specific: filters duplicate messages from redundant connections |

### Connection Pool Architecture

Production-grade active-active with deduplication:

```
┌─────────────────────────────────────────────────────────────────┐
│                    ConnectionPool                               │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                  Shard Manager                            │  │
│  │  - Divides subscriptions into N shards                    │  │
│  │  - Each shard has primary + backup connection             │  │
│  │  - Staggered connection lifecycle (no simultaneous expiry)│  │
│  └──────────────────────────────────────────────────────────┘  │
│                              │                                  │
│         ┌────────────────────┼────────────────────┐             │
│         ▼                    ▼                    ▼             │
│   ┌───────────┐        ┌───────────┐        ┌───────────┐      │
│   │ Shard 0   │        │ Shard 1   │        │ Shard 2   │      │
│   │ Conn A, B │        │ Conn C, D │        │ Conn E, F │      │
│   └─────┬─────┘        └─────┬─────┘        └─────┬─────┘      │
│         │                    │                    │             │
│         └────────────────────┼────────────────────┘             │
│                              ▼                                  │
│                    ┌─────────────────┐                          │
│                    │   Deduplicator  │                          │
│                    │ key: (token,    │                          │
│                    │       hash)     │                          │
│                    └────────┬────────┘                          │
│                             ▼                                   │
│                    Single MarketEvent stream                    │
└─────────────────────────────────────────────────────────────────┘
```

### Connection Lifecycle (Zero-Gap Failover)

```
Timeline for Shard with 2 connections:

T=0      Conn A connects, subscribes to shard's tokens
         Conn A: [████████████████████████████████████
         Conn B:

T=60s    Conn B connects (stagger_offset), subscribes
         Conn A: [████████████████████████████████████
         Conn B:                 [████████████████████

T=90s    Preemptive: Conn A' connects (30s before A expires)
         Conn A: [████████████████████████████████████──┐
         Conn B:                 [████████████████████████
         Conn A':                                    [███

T=120s   Conn A closes (server TTL), A' already active
         Conn A:  ────────────────────────────────────X
         Conn B:                 [███████████████████████
         Conn A':                                    [███

Result: Zero data gaps, always 2 active connections per shard
```

## Trait Abstractions

### MarketScorer (Exchange-Specific)

```rust
#[async_trait]
pub trait MarketScorer: Send + Sync {
    /// Score a market (higher = better opportunity potential)
    async fn score(&self, market: &MarketInfo) -> MarketScore;

    /// Batch scoring for efficiency
    async fn score_batch(&self, markets: &[MarketInfo]) -> Vec<MarketScore> {
        futures::future::join_all(markets.iter().map(|m| self.score(m))).await
    }
}

#[derive(Debug, Clone)]
pub struct MarketScore {
    pub market_id: MarketId,
    pub factors: ScoreFactors,
    pub composite: f64,
}

#[derive(Debug, Clone, Default)]
pub struct ScoreFactors {
    pub liquidity: f64,      // 0-1: Volume / depth
    pub spread: f64,         // 0-1: Tighter = higher
    pub opportunity: f64,    // 0-1: Historical arb rate
    pub outcome_count: f64,  // 0-1: Multi-outcome bonus
    pub activity: f64,       // 0-1: Recent trade frequency
}
```

### SubscriptionManager (Exchange-Agnostic)

```rust
pub trait SubscriptionManager: Send + Sync {
    /// Add markets to subscription pool (queued by priority)
    fn enqueue(&mut self, markets: Vec<(MarketId, MarketScore)>);

    /// Get currently active subscriptions
    fn active_subscriptions(&self) -> &[TokenId];

    /// Request to add N more subscriptions (from priority queue)
    async fn expand(&mut self, count: usize) -> Result<Vec<TokenId>>;

    /// Request to drop N lowest-priority subscriptions
    async fn contract(&mut self, count: usize) -> Result<Vec<TokenId>>;

    /// Handle connection lifecycle events
    async fn on_connection_event(&mut self, event: ConnectionEvent);
}
```

### AdaptiveGovernor (Exchange-Agnostic)

```rust
pub trait AdaptiveGovernor: Send + Sync {
    /// Record a processing latency sample
    fn record_latency(&mut self, latency: Duration);

    /// Record message throughput
    fn record_throughput(&mut self, messages_per_sec: f64);

    /// Get current scaling recommendation
    fn recommendation(&self) -> ScalingRecommendation;

    /// Update resource budget
    fn set_resource_budget(&mut self, budget: ResourceBudget);
}

#[derive(Debug, Clone)]
pub enum ScalingRecommendation {
    Expand { suggested_count: usize },
    Hold,
    Contract { suggested_count: usize },
}
```

### MessageDeduplicator (Exchange-Specific)

```rust
pub trait MessageDeduplicator: Send + Sync {
    /// Returns true if this message is a duplicate
    fn is_duplicate(&mut self, event: &MarketEvent) -> bool;

    /// Clear old entries
    fn gc(&mut self);
}
```

### ExchangeConfig Extensions

```rust
pub trait ExchangeConfig: Send + Sync {
    // Existing methods...
    fn parse_markets(&self, raw: &[MarketInfo]) -> Vec<Market>;

    // New: Provide default config values
    fn default_connection_config(&self) -> ConnectionConfig;
    fn default_dedup_config(&self) -> DedupConfig;
    fn default_filter_config(&self) -> MarketFilterConfig;
    fn default_scoring_config(&self) -> ScoringConfig;

    // New: Validate exchange-specific config
    fn validate_config(&self, config: &ExchangeSpecificConfig) -> Result<()>;

    // New: Exchange-imposed limits
    fn max_connections(&self) -> usize;
    fn max_subscriptions_per_connection(&self) -> usize;
}
```

## Configuration Schema

### Layer 1: System-Level (Exchange-Agnostic)

```toml
profile = "production"  # "local" | "production" | "custom"

[resources]
auto_detect = true
memory_usage_target = 0.80
cpu_usage_target = 0.70
# Or explicit:
# max_memory_mb = 4096
# worker_threads = 8

[governor]
enabled = true

[governor.latency]
target_p50_ms = 10
target_p95_ms = 50
target_p99_ms = 100
max_p99_ms = 200

[governor.scaling]
check_interval_secs = 10
expand_threshold = 0.70
contract_threshold = 1.20
expand_step = 50
contract_step = 100
cooldown_secs = 60
```

### Layer 2: Exchange-Level (Exchange-Specific)

```toml
exchange = "polymarket"

[exchange_config]
type = "polymarket"
environment = "mainnet"

[exchange_config.connections]
ws_url = "wss://ws-subscriptions-clob.polymarket.com/ws/market"
api_url = "https://clob.polymarket.com"
connection_ttl_secs = 120
preemptive_reconnect_secs = 30
max_connections = 10
subscriptions_per_connection = 500

[exchange_config.dedup]
enabled = true
strategy = "hash"
fallback = "timestamp"
cache_ttl_secs = 5
max_cache_entries = 100000

[exchange_config.market_filter]
max_markets = 500
max_subscriptions = 2000
min_volume_24h = 1000.0
min_liquidity = 500.0
max_spread_pct = 0.10
include_binary = true
include_multi_outcome = true
max_outcomes = 20

[exchange_config.scoring]
[exchange_config.scoring.weights]
liquidity = 0.30
spread = 0.20
opportunity = 0.25
outcome_count = 0.15
activity = 0.10

[exchange_config.scoring.outcome_bonus]
binary = 1.0
three_to_five = 1.5
six_plus = 2.0
```

### Layer 3: Connection Pool

```toml
[connection_pool]
num_shards = 3
connections_per_shard = 2
stagger_offset_secs = 60
health_check_interval_secs = 5
max_silent_secs = 10
```

### Profile Defaults

| Setting | `local` | `production` |
|---------|---------|--------------|
| `max_markets` | 50 | 500 |
| `max_subscriptions` | 200 | 2000 |
| `num_shards` | 1 | 3 |
| `connections_per_shard` | 1 | 2 |
| `governor.enabled` | false | true |
| `auto_detect` | false | true |
| `worker_threads` | 2 | auto |

## Data Flow

### Startup Flow

1. Load config (profile defaults → file → env overrides)
2. Auto-detect resources (if enabled) → `ResourceBudget`
3. Create exchange components via `ExchangeFactory`
4. Fetch all markets → Filter → Score → Sort by priority
5. Initialize `SubscriptionManager` with priority queue
6. Initialize `ConnectionPool` with shards
7. Start `AdaptiveGovernor` monitoring loop
8. Enter main event loop

### Event Loop

```
Connection 1 ──┐
Connection 2 ──┼──▶ Message Muxer ──▶ Deduplicator ──▶ Latency Instrumenter
Connection N ──┘                              │
                                              ▼
                                       OrderBook Cache
                                              │
                                              ▼
                                       Strategy Registry
                                              │
                                    ┌─────────┴─────────┐
                                    │                   │
                              No opportunity    Opportunity found
                                    │                   │
                                    ▼                   ▼
                                 (done)          Risk Manager
                                                       │
                                                 ┌─────┴─────┐
                                                 │           │
                                             Rejected   Approved
                                                 │           │
                                                 ▼           ▼
                                              (log)      Executor
```

### Governor Loop (Parallel)

```
Every check_interval_secs:

1. Collect metrics (latency percentiles, messages/sec, memory)
2. Evaluate against thresholds
   - p95 < target * expand_threshold  → EXPAND
   - p95 > target * contract_threshold → CONTRACT
   - otherwise → HOLD
3. Check cooldown (skip if within cooldown period)
4. Signal SubscriptionManager (expand or contract)
```

## Error Handling

### Error Categories

| Category | Examples | Response | Recovery |
|----------|----------|----------|----------|
| Connection | WS disconnect, timeout | Retry with backoff | Automatic via ConnectionPool |
| Exchange | Rate limit, auth failure | Back off, alert | Configurable retry policy |
| Data | Parse error, missing field | Log, skip message | Continue processing |
| Resource | OOM, CPU saturated | Contract subscriptions | Governor-driven |
| Config | Invalid values | Fail fast at startup | User fixes config |
| Execution | Order rejected | Log, notify, continue | Don't retry same opp |

### Circuit Breaker States

```
         success    ┌─────────┐
        ┌──────────▶│  CLOSED │◀──────────┐
        │           └────┬────┘           │
        │                │ failure        │ success after
        │                │ threshold      │ half-open test
        │                ▼                │
        │           ┌─────────┐           │
        │           │  OPEN   │───────────┤
        │           └────┬────┘           │
        │                │ cooldown       │
        │                │ expires        │
        │                ▼                │
        │           ┌───────────┐         │
        └───────────│ HALF-OPEN │─────────┘
          failure   └───────────┘
```

### Graceful Degradation Levels

| Level | State | Behavior |
|-------|-------|----------|
| 0 | HEALTHY | All shards active, full coverage |
| 1 | DEGRADED | Some shards reduced, high-priority maintained |
| 2 | IMPAIRED | Minimal shards, only top-tier markets |
| 3 | SURVIVAL | Single connection, detection only |
| 4 | OFFLINE | All connections failed, await intervention |

## Testing Strategy

### Unit Tests

| Component | What to test |
|-----------|--------------|
| `MarketScorer` | Score calculation, weight application |
| `Deduplicator` | Hash detection, TTL expiry, GC |
| `AdaptiveGovernor` | Threshold logic, cooldown, hysteresis |
| `SubscriptionManager` | Priority queue ordering, expand/contract |
| `ConnectionPool` | Shard assignment, stagger timing |

### Integration Tests

| Test | Scenario |
|------|----------|
| `pool_handles_disconnect` | Connection dies, backup takes over |
| `governor_contracts_under_load` | High latency triggers contraction |
| `dedup_across_connections` | Same message from both, only one passes |
| `full_lifecycle_rotation` | Preemptive reconnect before TTL |

### Simulation Tests

- `simulation_news_spike`: 10x message rate spike, verify adaptation
- `simulation_gradual_growth`: Slow ramp up, verify expansion
- `simulation_resource_pressure`: Memory limit hit, verify contraction

### Property-Based Tests

- Deduplicator never drops unique messages
- SubscriptionManager never exceeds limits
- Governor recommendations are monotonic within evaluation window

### E2E Tests (Testnet)

- Run against Polymarket testnet for 5 minutes
- Verify messages received, no errors, latency within bounds

### Chaos Tests

| Test | Injects | Verifies |
|------|---------|----------|
| `chaos_connection_flapping` | Random disconnects | System stays stable |
| `chaos_slow_network` | 500ms latency | Governor adapts |
| `chaos_memory_pressure` | Limit heap | Graceful degradation |

## Implementation Plan

### Phase 1: Core Abstractions
- [ ] Define trait interfaces
- [ ] Implement `MarketScore` and `ScoreFactors`
- [ ] Implement `SubscriptionManager` (in-memory priority queue)
- [ ] Add configuration schema and parsing

### Phase 2: Connection Pool
- [ ] Implement `ConnectionPool` with sharding
- [ ] Implement staggered lifecycle management
- [ ] Implement `MessageDeduplicator` for Polymarket
- [ ] Add preemptive reconnection logic

### Phase 3: Adaptive Governor
- [ ] Implement latency/throughput metrics collection
- [ ] Implement `AdaptiveGovernor` with threshold logic
- [ ] Wire governor to subscription manager
- [ ] Add resource auto-detection

### Phase 4: Exchange Integration
- [ ] Implement `PolymarketScorer`
- [ ] Extend `PolymarketConfig` with new settings
- [ ] Add market filtering logic
- [ ] Verify dedup with real Polymarket hash field

### Phase 5: Testing & Hardening
- [ ] Unit tests for all components
- [ ] Integration tests for component interactions
- [ ] Simulation tests for adaptive behavior
- [ ] E2E tests against testnet
- [ ] Chaos tests for failure scenarios

## Open Questions

1. **Polymarket connection limits**: Need to verify max connections per IP/account
2. **Hash field reliability**: Need to confirm `hash` is always populated in WebSocket messages
3. **Historical opportunity data**: How to bootstrap scoring without historical data?
4. **Multi-exchange**: Should sharding be per-exchange or global?

## Appendix: Dedup Key Availability

From `messages.rs`:

```rust
pub struct PolymarketBookMessage {
    pub asset_id: String,
    pub timestamp: Option<String>,  // Dedup key #1
    pub hash: Option<String>,       // Dedup key #2 (preferred)
    pub bids: Vec<...>,
    pub asks: Vec<...>,
}
```

Recommended dedup key: `(asset_id, hash)` with fallback to `(asset_id, timestamp)`.
