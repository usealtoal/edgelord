# Adaptive Subscription Management Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement a production-grade subscription management system that maximizes arbitrage opportunity coverage while maintaining low latency through adaptive scaling and zero-gap connection failover.

**Architecture:** Three new trait abstractions (`MarketScorer`, `SubscriptionManager`, `AdaptiveGovernor`) plus a sharded `ConnectionPool` with active-active redundancy. Exchange-specific implementations (Polymarket) implement scoring and deduplication. Configuration supports profiles (local/production) with resource auto-detection.

**Tech Stack:** Rust, tokio (async), dashmap (concurrent collections), metrics crate (instrumentation)

---

## Phase 1: Core Domain Types

### Task 1.1: Add Score Types

**Files:**
- Create: `src/core/domain/score.rs`
- Modify: `src/core/domain/mod.rs`

**Step 1: Create score module with ScoreFactors and MarketScore**

```rust
// src/core/domain/score.rs
//! Market scoring types for subscription prioritization.

use crate::core::domain::MarketId;

/// Individual factors contributing to a market's priority score.
#[derive(Debug, Clone, Default)]
pub struct ScoreFactors {
    /// Liquidity score (0.0-1.0): Order book depth and volume.
    pub liquidity: f64,
    /// Spread score (0.0-1.0): Tighter spreads score higher.
    pub spread: f64,
    /// Opportunity score (0.0-1.0): Historical arbitrage frequency.
    pub opportunity: f64,
    /// Outcome count score (0.0-1.0): Multi-outcome markets score higher.
    pub outcome_count: f64,
    /// Activity score (0.0-1.0): Recent trading activity.
    pub activity: f64,
}

impl ScoreFactors {
    /// Create new score factors with all values set.
    #[must_use]
    pub fn new(
        liquidity: f64,
        spread: f64,
        opportunity: f64,
        outcome_count: f64,
        activity: f64,
    ) -> Self {
        Self {
            liquidity,
            spread,
            opportunity,
            outcome_count,
            activity,
        }
    }

    /// Compute weighted composite score.
    #[must_use]
    pub fn composite(&self, weights: &ScoreWeights) -> f64 {
        self.liquidity * weights.liquidity
            + self.spread * weights.spread
            + self.opportunity * weights.opportunity
            + self.outcome_count * weights.outcome_count
            + self.activity * weights.activity
    }
}

/// Configurable weights for score factors.
#[derive(Debug, Clone)]
pub struct ScoreWeights {
    pub liquidity: f64,
    pub spread: f64,
    pub opportunity: f64,
    pub outcome_count: f64,
    pub activity: f64,
}

impl Default for ScoreWeights {
    fn default() -> Self {
        Self {
            liquidity: 0.30,
            spread: 0.20,
            opportunity: 0.25,
            outcome_count: 0.15,
            activity: 0.10,
        }
    }
}

/// A scored market with priority for subscription.
#[derive(Debug, Clone)]
pub struct MarketScore {
    /// The market being scored.
    pub market_id: MarketId,
    /// Individual factor scores.
    pub factors: ScoreFactors,
    /// Weighted composite score (higher = better).
    pub composite: f64,
}

impl MarketScore {
    /// Create a new market score.
    #[must_use]
    pub fn new(market_id: MarketId, factors: ScoreFactors, composite: f64) -> Self {
        Self {
            market_id,
            factors,
            composite,
        }
    }

    /// Create from factors using provided weights.
    #[must_use]
    pub fn from_factors(market_id: MarketId, factors: ScoreFactors, weights: &ScoreWeights) -> Self {
        let composite = factors.composite(weights);
        Self::new(market_id, factors, composite)
    }
}

impl PartialEq for MarketScore {
    fn eq(&self, other: &Self) -> bool {
        self.market_id == other.market_id
    }
}

impl Eq for MarketScore {}

impl PartialOrd for MarketScore {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MarketScore {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher composite = higher priority (reverse for max-heap behavior)
        self.composite
            .partial_cmp(&other.composite)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_factors_composite() {
        let factors = ScoreFactors::new(1.0, 1.0, 1.0, 1.0, 1.0);
        let weights = ScoreWeights::default();
        let composite = factors.composite(&weights);
        assert!((composite - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_score_factors_weighted() {
        let factors = ScoreFactors::new(0.5, 0.5, 0.5, 0.5, 0.5);
        let weights = ScoreWeights::default();
        let composite = factors.composite(&weights);
        assert!((composite - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_market_score_ordering() {
        let high = MarketScore::new(
            MarketId::from("high"),
            ScoreFactors::default(),
            0.9,
        );
        let low = MarketScore::new(
            MarketId::from("low"),
            ScoreFactors::default(),
            0.1,
        );
        assert!(high > low);
    }

    #[test]
    fn test_market_score_from_factors() {
        let factors = ScoreFactors::new(1.0, 1.0, 1.0, 1.0, 1.0);
        let weights = ScoreWeights::default();
        let score = MarketScore::from_factors(
            MarketId::from("test"),
            factors,
            &weights,
        );
        assert!((score.composite - 1.0).abs() < f64::EPSILON);
    }
}
```

**Step 2: Export from domain module**

Add to `src/core/domain/mod.rs`:
```rust
mod score;
pub use score::{MarketScore, ScoreFactors, ScoreWeights};
```

**Step 3: Run tests**

Run: `cargo test score`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/core/domain/score.rs src/core/domain/mod.rs
git commit -m "feat(domain): add market scoring types"
```

---

### Task 1.2: Add Resource Budget Types

**Files:**
- Create: `src/core/domain/resource.rs`
- Modify: `src/core/domain/mod.rs`

**Step 1: Create resource module**

```rust
// src/core/domain/resource.rs
//! Resource budget types for adaptive scaling.

/// Resource budget for the subscription system.
#[derive(Debug, Clone)]
pub struct ResourceBudget {
    /// Maximum memory in bytes.
    pub max_memory_bytes: u64,
    /// Number of worker threads available.
    pub worker_threads: usize,
    /// Target memory utilization (0.0-1.0).
    pub memory_target: f64,
    /// Target CPU utilization (0.0-1.0).
    pub cpu_target: f64,
}

impl ResourceBudget {
    /// Create a new resource budget.
    #[must_use]
    pub fn new(
        max_memory_bytes: u64,
        worker_threads: usize,
        memory_target: f64,
        cpu_target: f64,
    ) -> Self {
        Self {
            max_memory_bytes,
            worker_threads,
            memory_target,
            cpu_target,
        }
    }

    /// Create a budget for local development.
    #[must_use]
    pub fn local() -> Self {
        Self {
            max_memory_bytes: 512 * 1024 * 1024, // 512 MB
            worker_threads: 2,
            memory_target: 0.50,
            cpu_target: 0.50,
        }
    }

    /// Create a budget for production.
    #[must_use]
    pub fn production() -> Self {
        Self {
            max_memory_bytes: 4 * 1024 * 1024 * 1024, // 4 GB
            worker_threads: 8,
            memory_target: 0.80,
            cpu_target: 0.70,
        }
    }

    /// Estimate maximum subscriptions based on memory budget.
    /// Assumes ~10KB per subscription (order book + metadata).
    #[must_use]
    pub fn estimate_max_subscriptions(&self) -> usize {
        let usable_memory = (self.max_memory_bytes as f64 * self.memory_target) as u64;
        let per_subscription = 10 * 1024; // 10 KB estimate
        (usable_memory / per_subscription) as usize
    }
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self::local()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_budget() {
        let budget = ResourceBudget::local();
        assert_eq!(budget.worker_threads, 2);
        assert_eq!(budget.max_memory_bytes, 512 * 1024 * 1024);
    }

    #[test]
    fn test_production_budget() {
        let budget = ResourceBudget::production();
        assert_eq!(budget.worker_threads, 8);
        assert_eq!(budget.max_memory_bytes, 4 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_estimate_max_subscriptions() {
        let budget = ResourceBudget::local();
        let max_subs = budget.estimate_max_subscriptions();
        // 512MB * 0.5 / 10KB = ~25,600
        assert!(max_subs > 20_000);
        assert!(max_subs < 30_000);
    }
}
```

**Step 2: Export from domain module**

Add to `src/core/domain/mod.rs`:
```rust
mod resource;
pub use resource::ResourceBudget;
```

**Step 3: Run tests**

Run: `cargo test resource`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/core/domain/resource.rs src/core/domain/mod.rs
git commit -m "feat(domain): add resource budget types"
```

---

### Task 1.3: Add Scaling Recommendation Types

**Files:**
- Create: `src/core/domain/scaling.rs`
- Modify: `src/core/domain/mod.rs`

**Step 1: Create scaling module**

```rust
// src/core/domain/scaling.rs
//! Scaling recommendation types for adaptive governance.

/// Recommendation from the adaptive governor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScalingRecommendation {
    /// System has headroom, can add subscriptions.
    Expand {
        /// Suggested number of subscriptions to add.
        suggested_count: usize,
    },
    /// System is within target range, maintain current state.
    Hold,
    /// System is overwhelmed, should reduce subscriptions.
    Contract {
        /// Suggested number of subscriptions to remove.
        suggested_count: usize,
    },
}

impl ScalingRecommendation {
    /// Create an expand recommendation.
    #[must_use]
    pub fn expand(count: usize) -> Self {
        Self::Expand {
            suggested_count: count,
        }
    }

    /// Create a contract recommendation.
    #[must_use]
    pub fn contract(count: usize) -> Self {
        Self::Contract {
            suggested_count: count,
        }
    }

    /// Check if this is an expand recommendation.
    #[must_use]
    pub const fn is_expand(&self) -> bool {
        matches!(self, Self::Expand { .. })
    }

    /// Check if this is a hold recommendation.
    #[must_use]
    pub const fn is_hold(&self) -> bool {
        matches!(self, Self::Hold)
    }

    /// Check if this is a contract recommendation.
    #[must_use]
    pub const fn is_contract(&self) -> bool {
        matches!(self, Self::Contract { .. })
    }

    /// Get the suggested count if applicable.
    #[must_use]
    pub const fn suggested_count(&self) -> Option<usize> {
        match self {
            Self::Expand { suggested_count } | Self::Contract { suggested_count } => {
                Some(*suggested_count)
            }
            Self::Hold => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand() {
        let rec = ScalingRecommendation::expand(50);
        assert!(rec.is_expand());
        assert!(!rec.is_hold());
        assert!(!rec.is_contract());
        assert_eq!(rec.suggested_count(), Some(50));
    }

    #[test]
    fn test_hold() {
        let rec = ScalingRecommendation::Hold;
        assert!(!rec.is_expand());
        assert!(rec.is_hold());
        assert!(!rec.is_contract());
        assert_eq!(rec.suggested_count(), None);
    }

    #[test]
    fn test_contract() {
        let rec = ScalingRecommendation::contract(100);
        assert!(!rec.is_expand());
        assert!(!rec.is_hold());
        assert!(rec.is_contract());
        assert_eq!(rec.suggested_count(), Some(100));
    }
}
```

**Step 2: Export from domain module**

Add to `src/core/domain/mod.rs`:
```rust
mod scaling;
pub use scaling::ScalingRecommendation;
```

**Step 3: Run tests**

Run: `cargo test scaling`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/core/domain/scaling.rs src/core/domain/mod.rs
git commit -m "feat(domain): add scaling recommendation types"
```

---

## Phase 2: Trait Abstractions

### Task 2.1: Add MarketScorer Trait

**Files:**
- Create: `src/core/exchange/scorer.rs`
- Modify: `src/core/exchange/mod.rs`

**Step 1: Create scorer trait module**

```rust
// src/core/exchange/scorer.rs
//! Market scoring trait for subscription prioritization.

use async_trait::async_trait;

use crate::core::domain::{MarketScore, ScoreWeights};
use crate::core::exchange::MarketInfo;
use crate::error::Result;

/// Scores markets for subscription priority.
///
/// Implementations are exchange-specific, as different exchanges
/// provide different data for scoring (volume, liquidity, etc.).
#[async_trait]
pub trait MarketScorer: Send + Sync {
    /// Score a single market.
    async fn score(&self, market: &MarketInfo) -> Result<MarketScore>;

    /// Score multiple markets (batch optimization).
    ///
    /// Default implementation scores individually.
    async fn score_batch(&self, markets: &[MarketInfo]) -> Result<Vec<MarketScore>> {
        let mut scores = Vec::with_capacity(markets.len());
        for market in markets {
            scores.push(self.score(market).await?);
        }
        Ok(scores)
    }

    /// Get the scoring weights used by this scorer.
    fn weights(&self) -> &ScoreWeights;

    /// Get the exchange name for logging.
    fn exchange_name(&self) -> &'static str;
}
```

**Step 2: Export from exchange module**

Add to `src/core/exchange/mod.rs`:
```rust
mod scorer;
pub use scorer::MarketScorer;
```

**Step 3: Run tests**

Run: `cargo build`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/core/exchange/scorer.rs src/core/exchange/mod.rs
git commit -m "feat(exchange): add MarketScorer trait"
```

---

### Task 2.2: Add MarketFilter Trait

**Files:**
- Create: `src/core/exchange/filter.rs`
- Modify: `src/core/exchange/mod.rs`

**Step 1: Create filter trait module**

```rust
// src/core/exchange/filter.rs
//! Market filtering trait for subscription eligibility.

use crate::core::exchange::MarketInfo;

/// Configuration for market filtering.
#[derive(Debug, Clone)]
pub struct MarketFilterConfig {
    /// Maximum number of markets to consider.
    pub max_markets: usize,
    /// Maximum total token subscriptions.
    pub max_subscriptions: usize,
    /// Minimum 24h volume (USD).
    pub min_volume_24h: f64,
    /// Minimum order book liquidity (USD).
    pub min_liquidity: f64,
    /// Maximum bid-ask spread (0.0-1.0).
    pub max_spread_pct: f64,
    /// Include binary (YES/NO) markets.
    pub include_binary: bool,
    /// Include multi-outcome markets (3+).
    pub include_multi_outcome: bool,
    /// Maximum outcomes per market.
    pub max_outcomes: usize,
}

impl Default for MarketFilterConfig {
    fn default() -> Self {
        Self {
            max_markets: 500,
            max_subscriptions: 2000,
            min_volume_24h: 1000.0,
            min_liquidity: 500.0,
            max_spread_pct: 0.10,
            include_binary: true,
            include_multi_outcome: true,
            max_outcomes: 20,
        }
    }
}

/// Filters markets for subscription eligibility.
///
/// Implementations are exchange-specific, as different exchanges
/// provide different metadata for filtering.
pub trait MarketFilter: Send + Sync {
    /// Check if a market passes all filter criteria.
    fn is_eligible(&self, market: &MarketInfo) -> bool;

    /// Filter a list of markets, returning only eligible ones.
    fn filter(&self, markets: &[MarketInfo]) -> Vec<MarketInfo> {
        markets
            .iter()
            .filter(|m| self.is_eligible(m))
            .cloned()
            .collect()
    }

    /// Get the filter configuration.
    fn config(&self) -> &MarketFilterConfig;

    /// Get the exchange name for logging.
    fn exchange_name(&self) -> &'static str;
}
```

**Step 2: Export from exchange module**

Add to `src/core/exchange/mod.rs`:
```rust
mod filter;
pub use filter::{MarketFilter, MarketFilterConfig};
```

**Step 3: Run tests**

Run: `cargo build`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/core/exchange/filter.rs src/core/exchange/mod.rs
git commit -m "feat(exchange): add MarketFilter trait"
```

---

### Task 2.3: Add MessageDeduplicator Trait

**Files:**
- Create: `src/core/exchange/dedup.rs`
- Modify: `src/core/exchange/mod.rs`

**Step 1: Create dedup trait module**

```rust
// src/core/exchange/dedup.rs
//! Message deduplication trait for redundant connections.

use crate::core::exchange::MarketEvent;

/// Configuration for message deduplication.
#[derive(Debug, Clone)]
pub struct DedupConfig {
    /// Enable deduplication.
    pub enabled: bool,
    /// Deduplication strategy.
    pub strategy: DedupStrategy,
    /// Cache TTL in seconds.
    pub cache_ttl_secs: u64,
    /// Maximum cache entries.
    pub max_cache_entries: usize,
}

/// Deduplication strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DedupStrategy {
    /// Use message hash field (preferred for Polymarket).
    #[default]
    Hash,
    /// Use timestamp field.
    Timestamp,
    /// Hash message content.
    Content,
}

impl Default for DedupConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            strategy: DedupStrategy::Hash,
            cache_ttl_secs: 5,
            max_cache_entries: 100_000,
        }
    }
}

/// Deduplicates messages from redundant connections.
///
/// Implementations are exchange-specific, as different exchanges
/// provide different deduplication keys (hash, sequence, timestamp).
pub trait MessageDeduplicator: Send + Sync {
    /// Check if a message is a duplicate.
    ///
    /// Returns `true` if the message has been seen before.
    /// Also records the message for future deduplication.
    fn is_duplicate(&self, event: &MarketEvent) -> bool;

    /// Clear expired entries from the cache.
    fn gc(&self);

    /// Get the number of entries in the cache.
    fn cache_size(&self) -> usize;

    /// Get the exchange name for logging.
    fn exchange_name(&self) -> &'static str;
}
```

**Step 2: Export from exchange module**

Add to `src/core/exchange/mod.rs`:
```rust
mod dedup;
pub use dedup::{DedupConfig, DedupStrategy, MessageDeduplicator};
```

**Step 3: Run tests**

Run: `cargo build`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/core/exchange/dedup.rs src/core/exchange/mod.rs
git commit -m "feat(exchange): add MessageDeduplicator trait"
```

---

### Task 2.4: Add AdaptiveGovernor Trait

**Files:**
- Create: `src/core/service/governor.rs`
- Modify: `src/core/service/mod.rs`

**Step 1: Create governor trait module**

```rust
// src/core/service/governor.rs
//! Adaptive governor trait for scaling decisions.

use std::time::Duration;

use crate::core::domain::{ResourceBudget, ScalingRecommendation};

/// Configuration for the adaptive governor.
#[derive(Debug, Clone)]
pub struct GovernorConfig {
    /// Enable adaptive scaling.
    pub enabled: bool,
    /// Latency targets.
    pub latency: LatencyTargets,
    /// Scaling behavior.
    pub scaling: ScalingConfig,
}

/// Latency targets for the governor.
#[derive(Debug, Clone)]
pub struct LatencyTargets {
    /// Target p50 latency.
    pub target_p50: Duration,
    /// Target p95 latency.
    pub target_p95: Duration,
    /// Target p99 latency.
    pub target_p99: Duration,
    /// Maximum p99 latency (triggers contraction).
    pub max_p99: Duration,
}

impl Default for LatencyTargets {
    fn default() -> Self {
        Self {
            target_p50: Duration::from_millis(10),
            target_p95: Duration::from_millis(50),
            target_p99: Duration::from_millis(100),
            max_p99: Duration::from_millis(200),
        }
    }
}

/// Scaling behavior configuration.
#[derive(Debug, Clone)]
pub struct ScalingConfig {
    /// How often to check metrics.
    pub check_interval: Duration,
    /// Expand if p95 < target * expand_threshold.
    pub expand_threshold: f64,
    /// Contract if p95 > target * contract_threshold.
    pub contract_threshold: f64,
    /// Subscriptions to add per expansion.
    pub expand_step: usize,
    /// Subscriptions to remove per contraction.
    pub contract_step: usize,
    /// Cooldown between scaling actions.
    pub cooldown: Duration,
    /// Ignore brief spikes shorter than this.
    pub hysteresis: Duration,
}

impl Default for ScalingConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(10),
            expand_threshold: 0.70,
            contract_threshold: 1.20,
            expand_step: 50,
            contract_step: 100,
            cooldown: Duration::from_secs(60),
            hysteresis: Duration::from_secs(30),
        }
    }
}

impl Default for GovernorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            latency: LatencyTargets::default(),
            scaling: ScalingConfig::default(),
        }
    }
}

/// Latency percentiles collected by the governor.
#[derive(Debug, Clone, Default)]
pub struct LatencyMetrics {
    pub p50: Duration,
    pub p95: Duration,
    pub p99: Duration,
    pub sample_count: usize,
}

/// Monitors performance and signals scaling decisions.
pub trait AdaptiveGovernor: Send + Sync {
    /// Record a processing latency sample.
    fn record_latency(&self, latency: Duration);

    /// Record message throughput.
    fn record_throughput(&self, messages_per_sec: f64);

    /// Get current latency metrics.
    fn latency_metrics(&self) -> LatencyMetrics;

    /// Get current scaling recommendation.
    fn recommendation(&self) -> ScalingRecommendation;

    /// Notify that a scaling action was taken.
    fn notify_scaled(&self);

    /// Update resource budget.
    fn set_resource_budget(&self, budget: ResourceBudget);

    /// Get the configuration.
    fn config(&self) -> &GovernorConfig;
}
```

**Step 2: Export from service module**

Add to `src/core/service/mod.rs`:
```rust
mod governor;
pub use governor::{
    AdaptiveGovernor, GovernorConfig, LatencyMetrics, LatencyTargets, ScalingConfig,
};
```

**Step 3: Run tests**

Run: `cargo build`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/core/service/governor.rs src/core/service/mod.rs
git commit -m "feat(service): add AdaptiveGovernor trait"
```

---

### Task 2.5: Add SubscriptionManager Trait

**Files:**
- Create: `src/core/service/subscription.rs`
- Modify: `src/core/service/mod.rs`

**Step 1: Create subscription manager trait module**

```rust
// src/core/service/subscription.rs
//! Subscription manager trait for managing active subscriptions.

use async_trait::async_trait;

use crate::core::domain::{MarketId, MarketScore, TokenId};
use crate::error::Result;

/// Events from the connection pool.
#[derive(Debug, Clone)]
pub enum ConnectionEvent {
    /// A connection was established.
    Connected { connection_id: usize },
    /// A connection was lost.
    Disconnected { connection_id: usize, reason: String },
    /// A shard became unhealthy.
    ShardUnhealthy { shard_id: usize },
    /// A shard recovered.
    ShardRecovered { shard_id: usize },
}

/// Manages subscription lifecycle and priority.
#[async_trait]
pub trait SubscriptionManager: Send + Sync {
    /// Add markets to the subscription pool (queued by priority).
    fn enqueue(&self, markets: Vec<MarketScore>);

    /// Get currently active token subscriptions.
    fn active_subscriptions(&self) -> Vec<TokenId>;

    /// Get the number of active subscriptions.
    fn active_count(&self) -> usize;

    /// Get the number of pending (queued) markets.
    fn pending_count(&self) -> usize;

    /// Request to add more subscriptions from the priority queue.
    ///
    /// Returns the token IDs that were added.
    async fn expand(&self, count: usize) -> Result<Vec<TokenId>>;

    /// Request to drop lowest-priority subscriptions.
    ///
    /// Returns the token IDs that were removed.
    async fn contract(&self, count: usize) -> Result<Vec<TokenId>>;

    /// Handle a connection lifecycle event.
    async fn on_connection_event(&self, event: ConnectionEvent) -> Result<()>;

    /// Check if a market is currently subscribed.
    fn is_subscribed(&self, market_id: &MarketId) -> bool;

    /// Get the maximum subscription limit.
    fn max_subscriptions(&self) -> usize;
}
```

**Step 2: Export from service module**

Add to `src/core/service/mod.rs`:
```rust
mod subscription;
pub use subscription::{ConnectionEvent, SubscriptionManager};
```

**Step 3: Run tests**

Run: `cargo build`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/core/service/subscription.rs src/core/service/mod.rs
git commit -m "feat(service): add SubscriptionManager trait"
```

---

## Phase 3: Configuration Schema

### Task 3.1: Add Profile and Resource Configuration

**Files:**
- Modify: `src/app/config.rs`

**Step 1: Add profile enum and resource config**

Add to `src/app/config.rs` after the imports:

```rust
/// Configuration profile with sensible defaults.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    /// Local development settings.
    #[default]
    Local,
    /// Production settings.
    Production,
    /// Custom settings (all values from config file).
    Custom,
}

/// Resource budget configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ResourceConfig {
    /// Auto-detect available resources.
    #[serde(default = "default_false")]
    pub auto_detect: bool,
    /// Maximum memory in MB (optional, auto-detected if not set).
    pub max_memory_mb: Option<u64>,
    /// Number of worker threads (optional, auto-detected if not set).
    pub worker_threads: Option<usize>,
    /// Target memory utilization (0.0-1.0).
    #[serde(default = "default_memory_target")]
    pub memory_usage_target: f64,
    /// Target CPU utilization (0.0-1.0).
    #[serde(default = "default_cpu_target")]
    pub cpu_usage_target: f64,
}

fn default_false() -> bool {
    false
}

fn default_memory_target() -> f64 {
    0.80
}

fn default_cpu_target() -> f64 {
    0.70
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            auto_detect: false,
            max_memory_mb: None,
            worker_threads: None,
            memory_usage_target: default_memory_target(),
            cpu_usage_target: default_cpu_target(),
        }
    }
}

impl ResourceConfig {
    /// Convert to a ResourceBudget, applying auto-detection if enabled.
    #[must_use]
    pub fn to_budget(&self) -> ResourceBudget {
        let max_memory_bytes = self
            .max_memory_mb
            .map(|mb| mb * 1024 * 1024)
            .unwrap_or_else(|| {
                if self.auto_detect {
                    // TODO: Actually detect system memory
                    4 * 1024 * 1024 * 1024 // 4 GB default
                } else {
                    512 * 1024 * 1024 // 512 MB default
                }
            });

        let worker_threads = self.worker_threads.unwrap_or_else(|| {
            if self.auto_detect {
                num_cpus::get()
            } else {
                2
            }
        });

        ResourceBudget::new(
            max_memory_bytes,
            worker_threads,
            self.memory_usage_target,
            self.cpu_usage_target,
        )
    }
}
```

**Step 2: Add `num_cpus` dependency**

Run: `cargo add num_cpus`

**Step 3: Add to Config struct**

Add these fields to the `Config` struct:

```rust
    /// Configuration profile.
    #[serde(default)]
    pub profile: Profile,
    /// Resource budget configuration.
    #[serde(default)]
    pub resources: ResourceConfig,
```

**Step 4: Add import for ResourceBudget**

Add to imports at top of file:
```rust
use crate::core::domain::ResourceBudget;
```

**Step 5: Run tests**

Run: `cargo build`
Expected: Compiles successfully

**Step 6: Commit**

```bash
git add src/app/config.rs Cargo.toml Cargo.lock
git commit -m "feat(config): add profile and resource configuration"
```

---

### Task 3.2: Add Governor Configuration

**Files:**
- Modify: `src/app/config.rs`

**Step 1: Add governor config structs**

Add to `src/app/config.rs`:

```rust
/// Adaptive governor configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct GovernorAppConfig {
    /// Enable adaptive scaling.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Latency targets.
    #[serde(default)]
    pub latency: LatencyTargetsConfig,
    /// Scaling behavior.
    #[serde(default)]
    pub scaling: ScalingAppConfig,
}

impl Default for GovernorAppConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            latency: LatencyTargetsConfig::default(),
            scaling: ScalingAppConfig::default(),
        }
    }
}

/// Latency targets configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct LatencyTargetsConfig {
    /// Target p50 latency in milliseconds.
    #[serde(default = "default_target_p50_ms")]
    pub target_p50_ms: u64,
    /// Target p95 latency in milliseconds.
    #[serde(default = "default_target_p95_ms")]
    pub target_p95_ms: u64,
    /// Target p99 latency in milliseconds.
    #[serde(default = "default_target_p99_ms")]
    pub target_p99_ms: u64,
    /// Maximum p99 latency in milliseconds.
    #[serde(default = "default_max_p99_ms")]
    pub max_p99_ms: u64,
}

fn default_target_p50_ms() -> u64 {
    10
}

fn default_target_p95_ms() -> u64 {
    50
}

fn default_target_p99_ms() -> u64 {
    100
}

fn default_max_p99_ms() -> u64 {
    200
}

impl Default for LatencyTargetsConfig {
    fn default() -> Self {
        Self {
            target_p50_ms: default_target_p50_ms(),
            target_p95_ms: default_target_p95_ms(),
            target_p99_ms: default_target_p99_ms(),
            max_p99_ms: default_max_p99_ms(),
        }
    }
}

/// Scaling behavior configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ScalingAppConfig {
    /// Check interval in seconds.
    #[serde(default = "default_check_interval_secs")]
    pub check_interval_secs: u64,
    /// Expand threshold multiplier.
    #[serde(default = "default_expand_threshold")]
    pub expand_threshold: f64,
    /// Contract threshold multiplier.
    #[serde(default = "default_contract_threshold")]
    pub contract_threshold: f64,
    /// Subscriptions to add per expansion.
    #[serde(default = "default_expand_step")]
    pub expand_step: usize,
    /// Subscriptions to remove per contraction.
    #[serde(default = "default_contract_step")]
    pub contract_step: usize,
    /// Cooldown in seconds.
    #[serde(default = "default_cooldown_secs")]
    pub cooldown_secs: u64,
}

fn default_check_interval_secs() -> u64 {
    10
}

fn default_expand_threshold() -> f64 {
    0.70
}

fn default_contract_threshold() -> f64 {
    1.20
}

fn default_expand_step() -> usize {
    50
}

fn default_contract_step() -> usize {
    100
}

fn default_cooldown_secs() -> u64 {
    60
}

impl Default for ScalingAppConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: default_check_interval_secs(),
            expand_threshold: default_expand_threshold(),
            contract_threshold: default_contract_threshold(),
            expand_step: default_expand_step(),
            contract_step: default_contract_step(),
            cooldown_secs: default_cooldown_secs(),
        }
    }
}
```

**Step 2: Add to Config struct**

Add this field to the `Config` struct:

```rust
    /// Adaptive governor configuration.
    #[serde(default)]
    pub governor: GovernorAppConfig,
```

**Step 3: Run tests**

Run: `cargo build`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/app/config.rs
git commit -m "feat(config): add governor configuration"
```

---

### Task 3.3: Add Connection Pool Configuration

**Files:**
- Modify: `src/app/config.rs`

**Step 1: Add connection pool config struct**

Add to `src/app/config.rs`:

```rust
/// Connection pool configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionPoolConfig {
    /// Number of subscription shards.
    #[serde(default = "default_num_shards")]
    pub num_shards: usize,
    /// Connections per shard (primary + backups).
    #[serde(default = "default_connections_per_shard")]
    pub connections_per_shard: usize,
    /// Stagger offset between connections in seconds.
    #[serde(default = "default_stagger_offset_secs")]
    pub stagger_offset_secs: u64,
    /// Health check interval in seconds.
    #[serde(default = "default_health_check_interval_secs")]
    pub health_check_interval_secs: u64,
    /// Maximum silent period before reconnect in seconds.
    #[serde(default = "default_max_silent_secs")]
    pub max_silent_secs: u64,
}

fn default_num_shards() -> usize {
    3
}

fn default_connections_per_shard() -> usize {
    2
}

fn default_stagger_offset_secs() -> u64 {
    60
}

fn default_health_check_interval_secs() -> u64 {
    5
}

fn default_max_silent_secs() -> u64 {
    10
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            num_shards: default_num_shards(),
            connections_per_shard: default_connections_per_shard(),
            stagger_offset_secs: default_stagger_offset_secs(),
            health_check_interval_secs: default_health_check_interval_secs(),
            max_silent_secs: default_max_silent_secs(),
        }
    }
}

impl ConnectionPoolConfig {
    /// Get local development defaults.
    #[must_use]
    pub fn local() -> Self {
        Self {
            num_shards: 1,
            connections_per_shard: 1,
            ..Default::default()
        }
    }

    /// Get production defaults.
    #[must_use]
    pub fn production() -> Self {
        Self::default()
    }
}
```

**Step 2: Add to Config struct**

Add this field to the `Config` struct:

```rust
    /// Connection pool configuration.
    #[serde(default)]
    pub connection_pool: ConnectionPoolConfig,
```

**Step 3: Run tests**

Run: `cargo build`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/app/config.rs
git commit -m "feat(config): add connection pool configuration"
```

---

### Task 3.4: Add Exchange-Specific Configuration Extensions

**Files:**
- Modify: `src/app/config.rs`

**Step 1: Extend PolymarketConfig with new sections**

Replace the `PolymarketConfig` struct with:

```rust
/// Polymarket exchange configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct PolymarketConfig {
    /// Environment: testnet or mainnet.
    #[serde(default)]
    pub environment: Environment,
    /// WebSocket URL for market data.
    #[serde(default = "default_polymarket_ws_url")]
    pub ws_url: String,
    /// REST API URL.
    #[serde(default = "default_polymarket_api_url")]
    pub api_url: String,
    /// Chain ID: 80002 for Amoy testnet, 137 for Polygon mainnet.
    #[serde(default = "default_polymarket_chain_id")]
    pub chain_id: u64,
    /// Connection settings.
    #[serde(default)]
    pub connections: PolymarketConnectionConfig,
    /// Market filter settings.
    #[serde(default)]
    pub market_filter: PolymarketFilterConfig,
    /// Scoring settings.
    #[serde(default)]
    pub scoring: PolymarketScoringConfig,
    /// Deduplication settings.
    #[serde(default)]
    pub dedup: PolymarketDedupConfig,
}

/// Polymarket connection configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct PolymarketConnectionConfig {
    /// Connection TTL in seconds (server-imposed).
    #[serde(default = "default_connection_ttl_secs")]
    pub connection_ttl_secs: u64,
    /// Preemptive reconnect before TTL in seconds.
    #[serde(default = "default_preemptive_reconnect_secs")]
    pub preemptive_reconnect_secs: u64,
    /// Maximum connections (exchange limit).
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
    /// Subscriptions per connection (exchange limit).
    #[serde(default = "default_subscriptions_per_connection")]
    pub subscriptions_per_connection: usize,
}

fn default_connection_ttl_secs() -> u64 {
    120
}

fn default_preemptive_reconnect_secs() -> u64 {
    30
}

fn default_max_connections() -> usize {
    10
}

fn default_subscriptions_per_connection() -> usize {
    500
}

impl Default for PolymarketConnectionConfig {
    fn default() -> Self {
        Self {
            connection_ttl_secs: default_connection_ttl_secs(),
            preemptive_reconnect_secs: default_preemptive_reconnect_secs(),
            max_connections: default_max_connections(),
            subscriptions_per_connection: default_subscriptions_per_connection(),
        }
    }
}

/// Polymarket market filter configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct PolymarketFilterConfig {
    /// Maximum markets to consider.
    #[serde(default = "default_max_markets")]
    pub max_markets: usize,
    /// Maximum total subscriptions.
    #[serde(default = "default_max_subscriptions")]
    pub max_subscriptions: usize,
    /// Minimum 24h volume in USD.
    #[serde(default = "default_min_volume_24h")]
    pub min_volume_24h: f64,
    /// Minimum liquidity in USD.
    #[serde(default = "default_min_liquidity")]
    pub min_liquidity: f64,
    /// Maximum spread percentage.
    #[serde(default = "default_max_spread_pct")]
    pub max_spread_pct: f64,
    /// Include binary markets.
    #[serde(default = "default_true")]
    pub include_binary: bool,
    /// Include multi-outcome markets.
    #[serde(default = "default_true")]
    pub include_multi_outcome: bool,
    /// Maximum outcomes per market.
    #[serde(default = "default_max_outcomes")]
    pub max_outcomes: usize,
}

fn default_max_markets() -> usize {
    500
}

fn default_max_subscriptions() -> usize {
    2000
}

fn default_min_volume_24h() -> f64 {
    1000.0
}

fn default_min_liquidity() -> f64 {
    500.0
}

fn default_max_spread_pct() -> f64 {
    0.10
}

fn default_max_outcomes() -> usize {
    20
}

impl Default for PolymarketFilterConfig {
    fn default() -> Self {
        Self {
            max_markets: default_max_markets(),
            max_subscriptions: default_max_subscriptions(),
            min_volume_24h: default_min_volume_24h(),
            min_liquidity: default_min_liquidity(),
            max_spread_pct: default_max_spread_pct(),
            include_binary: true,
            include_multi_outcome: true,
            max_outcomes: default_max_outcomes(),
        }
    }
}

/// Polymarket scoring configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct PolymarketScoringConfig {
    /// Scoring weights.
    #[serde(default)]
    pub weights: ScoringWeightsConfig,
    /// Outcome count bonuses.
    #[serde(default)]
    pub outcome_bonus: OutcomeBonusConfig,
}

impl Default for PolymarketScoringConfig {
    fn default() -> Self {
        Self {
            weights: ScoringWeightsConfig::default(),
            outcome_bonus: OutcomeBonusConfig::default(),
        }
    }
}

/// Scoring weights configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ScoringWeightsConfig {
    pub liquidity: f64,
    pub spread: f64,
    pub opportunity: f64,
    pub outcome_count: f64,
    pub activity: f64,
}

impl Default for ScoringWeightsConfig {
    fn default() -> Self {
        Self {
            liquidity: 0.30,
            spread: 0.20,
            opportunity: 0.25,
            outcome_count: 0.15,
            activity: 0.10,
        }
    }
}

/// Outcome count bonus configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct OutcomeBonusConfig {
    /// Bonus for binary markets.
    #[serde(default = "default_binary_bonus")]
    pub binary: f64,
    /// Bonus for 3-5 outcome markets.
    #[serde(default = "default_three_to_five_bonus")]
    pub three_to_five: f64,
    /// Bonus for 6+ outcome markets.
    #[serde(default = "default_six_plus_bonus")]
    pub six_plus: f64,
}

fn default_binary_bonus() -> f64 {
    1.0
}

fn default_three_to_five_bonus() -> f64 {
    1.5
}

fn default_six_plus_bonus() -> f64 {
    2.0
}

impl Default for OutcomeBonusConfig {
    fn default() -> Self {
        Self {
            binary: default_binary_bonus(),
            three_to_five: default_three_to_five_bonus(),
            six_plus: default_six_plus_bonus(),
        }
    }
}

/// Polymarket deduplication configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct PolymarketDedupConfig {
    /// Enable deduplication.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Deduplication strategy.
    #[serde(default)]
    pub strategy: DedupStrategyConfig,
    /// Fallback strategy if primary unavailable.
    #[serde(default = "default_fallback_strategy")]
    pub fallback: DedupStrategyConfig,
    /// Cache TTL in seconds.
    #[serde(default = "default_cache_ttl_secs")]
    pub cache_ttl_secs: u64,
    /// Maximum cache entries.
    #[serde(default = "default_max_cache_entries")]
    pub max_cache_entries: usize,
}

/// Deduplication strategy configuration.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DedupStrategyConfig {
    #[default]
    Hash,
    Timestamp,
    Content,
}

fn default_fallback_strategy() -> DedupStrategyConfig {
    DedupStrategyConfig::Timestamp
}

fn default_cache_ttl_secs() -> u64 {
    5
}

fn default_max_cache_entries() -> usize {
    100_000
}

impl Default for PolymarketDedupConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            strategy: DedupStrategyConfig::Hash,
            fallback: default_fallback_strategy(),
            cache_ttl_secs: default_cache_ttl_secs(),
            max_cache_entries: default_max_cache_entries(),
        }
    }
}
```

**Step 2: Update PolymarketConfig Default impl**

```rust
impl Default for PolymarketConfig {
    fn default() -> Self {
        Self {
            environment: Environment::default(),
            ws_url: default_polymarket_ws_url(),
            api_url: default_polymarket_api_url(),
            chain_id: default_polymarket_chain_id(),
            connections: PolymarketConnectionConfig::default(),
            market_filter: PolymarketFilterConfig::default(),
            scoring: PolymarketScoringConfig::default(),
            dedup: PolymarketDedupConfig::default(),
        }
    }
}
```

**Step 3: Run tests**

Run: `cargo build`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/app/config.rs
git commit -m "feat(config): add exchange-specific configuration extensions"
```

---

## Phase 4: Polymarket Implementations

### Task 4.1: Implement PolymarketScorer

**Files:**
- Create: `src/core/exchange/polymarket/scorer.rs`
- Modify: `src/core/exchange/polymarket/mod.rs`

**Step 1: Create Polymarket scorer implementation**

```rust
// src/core/exchange/polymarket/scorer.rs
//! Polymarket market scorer implementation.

use async_trait::async_trait;

use crate::app::config::PolymarketScoringConfig;
use crate::core::domain::{MarketId, MarketScore, ScoreFactors, ScoreWeights};
use crate::core::exchange::{MarketInfo, MarketScorer};
use crate::error::Result;

/// Polymarket market scorer.
pub struct PolymarketScorer {
    weights: ScoreWeights,
    outcome_bonus: OutcomeBonus,
}

struct OutcomeBonus {
    binary: f64,
    three_to_five: f64,
    six_plus: f64,
}

impl PolymarketScorer {
    /// Create a new Polymarket scorer with the given configuration.
    #[must_use]
    pub fn new(config: &PolymarketScoringConfig) -> Self {
        Self {
            weights: ScoreWeights {
                liquidity: config.weights.liquidity,
                spread: config.weights.spread,
                opportunity: config.weights.opportunity,
                outcome_count: config.weights.outcome_count,
                activity: config.weights.activity,
            },
            outcome_bonus: OutcomeBonus {
                binary: config.outcome_bonus.binary,
                three_to_five: config.outcome_bonus.three_to_five,
                six_plus: config.outcome_bonus.six_plus,
            },
        }
    }

    /// Calculate outcome count score based on number of outcomes.
    fn outcome_score(&self, outcome_count: usize) -> f64 {
        let bonus = match outcome_count {
            2 => self.outcome_bonus.binary,
            3..=5 => self.outcome_bonus.three_to_five,
            _ => self.outcome_bonus.six_plus,
        };
        // Normalize to 0-1 range (assuming max bonus is 2.0)
        (bonus / 2.0).min(1.0)
    }
}

#[async_trait]
impl MarketScorer for PolymarketScorer {
    async fn score(&self, market: &MarketInfo) -> Result<MarketScore> {
        // For now, use simple heuristics since we don't have full market data.
        // In production, this would fetch volume/liquidity from the API.
        let outcome_count = market.outcomes.len();

        let factors = ScoreFactors::new(
            0.5,                                // liquidity: placeholder
            0.5,                                // spread: placeholder
            0.5,                                // opportunity: placeholder
            self.outcome_score(outcome_count),  // outcome_count: based on actual count
            0.5,                                // activity: placeholder
        );

        Ok(MarketScore::from_factors(
            MarketId::from(market.id.clone()),
            factors,
            &self.weights,
        ))
    }

    fn weights(&self) -> &ScoreWeights {
        &self.weights
    }

    fn exchange_name(&self) -> &'static str {
        "polymarket"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::exchange::OutcomeInfo;

    fn make_market(id: &str, outcomes: usize) -> MarketInfo {
        let outcome_infos: Vec<_> = (0..outcomes)
            .map(|i| OutcomeInfo {
                token_id: format!("token-{i}"),
                name: format!("Outcome {i}"),
            })
            .collect();

        MarketInfo {
            id: id.to_string(),
            question: "Test question?".to_string(),
            outcomes: outcome_infos,
            active: true,
        }
    }

    #[tokio::test]
    async fn test_score_binary_market() {
        let config = PolymarketScoringConfig::default();
        let scorer = PolymarketScorer::new(&config);

        let market = make_market("binary", 2);
        let score = scorer.score(&market).await.unwrap();

        assert_eq!(score.market_id.as_str(), "binary");
        assert!(score.composite > 0.0);
    }

    #[tokio::test]
    async fn test_score_multi_outcome_market() {
        let config = PolymarketScoringConfig::default();
        let scorer = PolymarketScorer::new(&config);

        let market = make_market("multi", 6);
        let score = scorer.score(&market).await.unwrap();

        assert_eq!(score.market_id.as_str(), "multi");
        // Multi-outcome should score higher on outcome_count factor
        assert!(score.factors.outcome_count > 0.5);
    }

    #[tokio::test]
    async fn test_outcome_bonus_applied() {
        let config = PolymarketScoringConfig::default();
        let scorer = PolymarketScorer::new(&config);

        let binary = make_market("binary", 2);
        let multi = make_market("multi", 6);

        let binary_score = scorer.score(&binary).await.unwrap();
        let multi_score = scorer.score(&multi).await.unwrap();

        // Multi-outcome should have higher outcome_count factor
        assert!(multi_score.factors.outcome_count > binary_score.factors.outcome_count);
    }
}
```

**Step 2: Export from polymarket module**

Add to `src/core/exchange/polymarket/mod.rs`:
```rust
mod scorer;
pub use scorer::PolymarketScorer;
```

**Step 3: Run tests**

Run: `cargo test polymarket::scorer`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/core/exchange/polymarket/scorer.rs src/core/exchange/polymarket/mod.rs
git commit -m "feat(polymarket): implement MarketScorer"
```

---

### Task 4.2: Implement PolymarketFilter

**Files:**
- Create: `src/core/exchange/polymarket/filter.rs`
- Modify: `src/core/exchange/polymarket/mod.rs`

**Step 1: Create Polymarket filter implementation**

```rust
// src/core/exchange/polymarket/filter.rs
//! Polymarket market filter implementation.

use crate::app::config::PolymarketFilterConfig;
use crate::core::exchange::{MarketFilter, MarketFilterConfig, MarketInfo};

/// Polymarket market filter.
pub struct PolymarketFilter {
    config: MarketFilterConfig,
}

impl PolymarketFilter {
    /// Create a new Polymarket filter with the given configuration.
    #[must_use]
    pub fn new(config: &PolymarketFilterConfig) -> Self {
        Self {
            config: MarketFilterConfig {
                max_markets: config.max_markets,
                max_subscriptions: config.max_subscriptions,
                min_volume_24h: config.min_volume_24h,
                min_liquidity: config.min_liquidity,
                max_spread_pct: config.max_spread_pct,
                include_binary: config.include_binary,
                include_multi_outcome: config.include_multi_outcome,
                max_outcomes: config.max_outcomes,
            },
        }
    }
}

impl MarketFilter for PolymarketFilter {
    fn is_eligible(&self, market: &MarketInfo) -> bool {
        // Check if market is active
        if !market.active {
            return false;
        }

        let outcome_count = market.outcomes.len();

        // Check outcome count bounds
        if outcome_count > self.config.max_outcomes {
            return false;
        }

        // Check market type inclusion
        let is_binary = outcome_count == 2;
        if is_binary && !self.config.include_binary {
            return false;
        }
        if !is_binary && !self.config.include_multi_outcome {
            return false;
        }

        // TODO: Check volume, liquidity, spread when we have that data
        // For now, pass through if active and within outcome bounds

        true
    }

    fn config(&self) -> &MarketFilterConfig {
        &self.config
    }

    fn exchange_name(&self) -> &'static str {
        "polymarket"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::exchange::OutcomeInfo;

    fn make_market(id: &str, outcomes: usize, active: bool) -> MarketInfo {
        let outcome_infos: Vec<_> = (0..outcomes)
            .map(|i| OutcomeInfo {
                token_id: format!("token-{i}"),
                name: format!("Outcome {i}"),
            })
            .collect();

        MarketInfo {
            id: id.to_string(),
            question: "Test question?".to_string(),
            outcomes: outcome_infos,
            active,
        }
    }

    #[test]
    fn test_filter_active_binary() {
        let config = PolymarketFilterConfig::default();
        let filter = PolymarketFilter::new(&config);

        let market = make_market("test", 2, true);
        assert!(filter.is_eligible(&market));
    }

    #[test]
    fn test_filter_inactive_market() {
        let config = PolymarketFilterConfig::default();
        let filter = PolymarketFilter::new(&config);

        let market = make_market("test", 2, false);
        assert!(!filter.is_eligible(&market));
    }

    #[test]
    fn test_filter_too_many_outcomes() {
        let mut config = PolymarketFilterConfig::default();
        config.max_outcomes = 5;
        let filter = PolymarketFilter::new(&config);

        let market = make_market("test", 10, true);
        assert!(!filter.is_eligible(&market));
    }

    #[test]
    fn test_filter_exclude_binary() {
        let mut config = PolymarketFilterConfig::default();
        config.include_binary = false;
        let filter = PolymarketFilter::new(&config);

        let market = make_market("test", 2, true);
        assert!(!filter.is_eligible(&market));
    }

    #[test]
    fn test_filter_exclude_multi_outcome() {
        let mut config = PolymarketFilterConfig::default();
        config.include_multi_outcome = false;
        let filter = PolymarketFilter::new(&config);

        let market = make_market("test", 5, true);
        assert!(!filter.is_eligible(&market));
    }

    #[test]
    fn test_filter_batch() {
        let config = PolymarketFilterConfig::default();
        let filter = PolymarketFilter::new(&config);

        let markets = vec![
            make_market("active", 2, true),
            make_market("inactive", 2, false),
            make_market("multi", 5, true),
        ];

        let filtered = filter.filter(&markets);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].id, "active");
        assert_eq!(filtered[1].id, "multi");
    }
}
```

**Step 2: Export from polymarket module**

Add to `src/core/exchange/polymarket/mod.rs`:
```rust
mod filter;
pub use filter::PolymarketFilter;
```

**Step 3: Run tests**

Run: `cargo test polymarket::filter`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/core/exchange/polymarket/filter.rs src/core/exchange/polymarket/mod.rs
git commit -m "feat(polymarket): implement MarketFilter"
```

---

### Task 4.3: Implement PolymarketDeduplicator

**Files:**
- Create: `src/core/exchange/polymarket/dedup.rs`
- Modify: `src/core/exchange/polymarket/mod.rs`

**Step 1: Add dashmap dependency**

Run: `cargo add dashmap`

**Step 2: Create Polymarket deduplicator implementation**

```rust
// src/core/exchange/polymarket/dedup.rs
//! Polymarket message deduplicator implementation.

use std::time::{Duration, Instant};

use dashmap::DashMap;

use crate::app::config::PolymarketDedupConfig;
use crate::core::exchange::{MarketEvent, MessageDeduplicator};

/// Polymarket message deduplicator using hash-based deduplication.
pub struct PolymarketDeduplicator {
    /// Cache of seen message keys with their insertion time.
    cache: DashMap<String, Instant>,
    /// Cache TTL.
    ttl: Duration,
    /// Maximum cache entries.
    max_entries: usize,
    /// Whether deduplication is enabled.
    enabled: bool,
}

impl PolymarketDeduplicator {
    /// Create a new Polymarket deduplicator with the given configuration.
    #[must_use]
    pub fn new(config: &PolymarketDedupConfig) -> Self {
        Self {
            cache: DashMap::new(),
            ttl: Duration::from_secs(config.cache_ttl_secs),
            max_entries: config.max_cache_entries,
            enabled: config.enabled,
        }
    }

    /// Generate a dedup key for a market event.
    fn make_key(event: &MarketEvent) -> Option<String> {
        match event {
            MarketEvent::OrderBookSnapshot { token_id, book } |
            MarketEvent::OrderBookDelta { token_id, book } => {
                // Use token_id + hash if available, otherwise content hash
                // For now, use a simple content-based key
                let bids_hash = book.bids().len();
                let asks_hash = book.asks().len();
                let best_bid = book.best_bid().map(|l| l.price.to_string()).unwrap_or_default();
                let best_ask = book.best_ask().map(|l| l.price.to_string()).unwrap_or_default();
                Some(format!("{}:{}:{}:{}:{}", token_id.as_str(), bids_hash, asks_hash, best_bid, best_ask))
            }
            _ => None,
        }
    }
}

impl MessageDeduplicator for PolymarketDeduplicator {
    fn is_duplicate(&self, event: &MarketEvent) -> bool {
        if !self.enabled {
            return false;
        }

        let Some(key) = Self::make_key(event) else {
            return false;
        };

        let now = Instant::now();

        // Check if we've seen this key recently
        if let Some(entry) = self.cache.get(&key) {
            if now.duration_since(*entry) < self.ttl {
                return true;
            }
        }

        // Enforce max entries (simple eviction: just check size)
        if self.cache.len() >= self.max_entries {
            self.gc();
        }

        // Record this key
        self.cache.insert(key, now);
        false
    }

    fn gc(&self) {
        let now = Instant::now();
        self.cache.retain(|_, inserted| now.duration_since(*inserted) < self.ttl);
    }

    fn cache_size(&self) -> usize {
        self.cache.len()
    }

    fn exchange_name(&self) -> &'static str {
        "polymarket"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::domain::{OrderBook, PriceLevel, TokenId};
    use rust_decimal_macros::dec;

    fn make_snapshot(token: &str, bid: &str, ask: &str) -> MarketEvent {
        let token_id = TokenId::from(token.to_string());
        let book = OrderBook::with_levels(
            token_id.clone(),
            vec![PriceLevel::new(bid.parse().unwrap(), dec!(100))],
            vec![PriceLevel::new(ask.parse().unwrap(), dec!(100))],
        );
        MarketEvent::OrderBookSnapshot { token_id, book }
    }

    #[test]
    fn test_first_message_not_duplicate() {
        let config = PolymarketDedupConfig::default();
        let dedup = PolymarketDeduplicator::new(&config);

        let event = make_snapshot("token1", "0.40", "0.60");
        assert!(!dedup.is_duplicate(&event));
    }

    #[test]
    fn test_same_message_is_duplicate() {
        let config = PolymarketDedupConfig::default();
        let dedup = PolymarketDeduplicator::new(&config);

        let event = make_snapshot("token1", "0.40", "0.60");
        assert!(!dedup.is_duplicate(&event));
        assert!(dedup.is_duplicate(&event));
    }

    #[test]
    fn test_different_message_not_duplicate() {
        let config = PolymarketDedupConfig::default();
        let dedup = PolymarketDeduplicator::new(&config);

        let event1 = make_snapshot("token1", "0.40", "0.60");
        let event2 = make_snapshot("token1", "0.41", "0.60"); // Different bid

        assert!(!dedup.is_duplicate(&event1));
        assert!(!dedup.is_duplicate(&event2));
    }

    #[test]
    fn test_different_token_not_duplicate() {
        let config = PolymarketDedupConfig::default();
        let dedup = PolymarketDeduplicator::new(&config);

        let event1 = make_snapshot("token1", "0.40", "0.60");
        let event2 = make_snapshot("token2", "0.40", "0.60");

        assert!(!dedup.is_duplicate(&event1));
        assert!(!dedup.is_duplicate(&event2));
    }

    #[test]
    fn test_disabled_never_duplicate() {
        let mut config = PolymarketDedupConfig::default();
        config.enabled = false;
        let dedup = PolymarketDeduplicator::new(&config);

        let event = make_snapshot("token1", "0.40", "0.60");
        assert!(!dedup.is_duplicate(&event));
        assert!(!dedup.is_duplicate(&event));
    }

    #[test]
    fn test_gc_removes_old_entries() {
        let mut config = PolymarketDedupConfig::default();
        config.cache_ttl_secs = 0; // Immediate expiry for testing
        let dedup = PolymarketDeduplicator::new(&config);

        let event = make_snapshot("token1", "0.40", "0.60");
        assert!(!dedup.is_duplicate(&event));

        // After GC with 0 TTL, entry should be removed
        dedup.gc();
        assert_eq!(dedup.cache_size(), 0);
    }
}
```

**Step 3: Export from polymarket module**

Add to `src/core/exchange/polymarket/mod.rs`:
```rust
mod dedup;
pub use dedup::PolymarketDeduplicator;
```

**Step 4: Run tests**

Run: `cargo test polymarket::dedup`
Expected: All tests pass

**Step 5: Commit**

```bash
git add src/core/exchange/polymarket/dedup.rs src/core/exchange/polymarket/mod.rs Cargo.toml Cargo.lock
git commit -m "feat(polymarket): implement MessageDeduplicator"
```

---

## Phase 5: Core Implementations

### Task 5.1: Implement PrioritySubscriptionManager

**Files:**
- Create: `src/core/service/priority_subscription.rs`
- Modify: `src/core/service/mod.rs`

**Step 1: Create priority subscription manager**

```rust
// src/core/service/priority_subscription.rs
//! Priority-based subscription manager implementation.

use std::collections::{BinaryHeap, HashSet};
use std::sync::RwLock;

use async_trait::async_trait;

use crate::core::domain::{Market, MarketId, MarketScore, TokenId};
use crate::core::service::{ConnectionEvent, SubscriptionManager};
use crate::error::Result;

/// Priority-based subscription manager.
///
/// Maintains a priority queue of markets and tracks active subscriptions.
pub struct PrioritySubscriptionManager {
    /// Priority queue of pending markets (max-heap by score).
    pending: RwLock<BinaryHeap<MarketScore>>,
    /// Currently active subscriptions by market ID.
    active_markets: RwLock<HashSet<MarketId>>,
    /// Token IDs for active subscriptions.
    active_tokens: RwLock<Vec<TokenId>>,
    /// Market ID to tokens mapping.
    market_tokens: RwLock<std::collections::HashMap<MarketId, Vec<TokenId>>>,
    /// Maximum subscriptions allowed.
    max_subscriptions: usize,
}

impl PrioritySubscriptionManager {
    /// Create a new priority subscription manager.
    #[must_use]
    pub fn new(max_subscriptions: usize) -> Self {
        Self {
            pending: RwLock::new(BinaryHeap::new()),
            active_markets: RwLock::new(HashSet::new()),
            active_tokens: RwLock::new(Vec::new()),
            market_tokens: RwLock::new(std::collections::HashMap::new()),
            max_subscriptions,
        }
    }

    /// Register market token mapping.
    pub fn register_market_tokens(&self, market_id: MarketId, tokens: Vec<TokenId>) {
        let mut market_tokens = self.market_tokens.write().unwrap();
        market_tokens.insert(market_id, tokens);
    }
}

#[async_trait]
impl SubscriptionManager for PrioritySubscriptionManager {
    fn enqueue(&self, markets: Vec<MarketScore>) {
        let mut pending = self.pending.write().unwrap();
        for score in markets {
            pending.push(score);
        }
    }

    fn active_subscriptions(&self) -> Vec<TokenId> {
        self.active_tokens.read().unwrap().clone()
    }

    fn active_count(&self) -> usize {
        self.active_tokens.read().unwrap().len()
    }

    fn pending_count(&self) -> usize {
        self.pending.read().unwrap().len()
    }

    async fn expand(&self, count: usize) -> Result<Vec<TokenId>> {
        let mut added_tokens = Vec::new();
        let mut pending = self.pending.write().unwrap();
        let mut active_markets = self.active_markets.write().unwrap();
        let mut active_tokens = self.active_tokens.write().unwrap();
        let market_tokens = self.market_tokens.read().unwrap();

        let mut markets_to_add = 0;
        while markets_to_add < count && !pending.is_empty() {
            if active_tokens.len() >= self.max_subscriptions {
                break;
            }

            if let Some(score) = pending.pop() {
                if active_markets.contains(&score.market_id) {
                    continue;
                }

                if let Some(tokens) = market_tokens.get(&score.market_id) {
                    if active_tokens.len() + tokens.len() <= self.max_subscriptions {
                        active_markets.insert(score.market_id);
                        for token in tokens {
                            active_tokens.push(token.clone());
                            added_tokens.push(token.clone());
                        }
                        markets_to_add += 1;
                    }
                }
            }
        }

        Ok(added_tokens)
    }

    async fn contract(&self, count: usize) -> Result<Vec<TokenId>> {
        // For simplicity, remove the most recently added (LIFO)
        // A more sophisticated approach would remove lowest-priority
        let mut removed_tokens = Vec::new();
        let mut active_tokens = self.active_tokens.write().unwrap();

        let to_remove = count.min(active_tokens.len());
        for _ in 0..to_remove {
            if let Some(token) = active_tokens.pop() {
                removed_tokens.push(token);
            }
        }

        Ok(removed_tokens)
    }

    async fn on_connection_event(&self, event: ConnectionEvent) -> Result<()> {
        match event {
            ConnectionEvent::Disconnected { connection_id, reason } => {
                tracing::warn!(connection_id, reason, "Connection lost");
            }
            ConnectionEvent::ShardUnhealthy { shard_id } => {
                tracing::warn!(shard_id, "Shard unhealthy");
            }
            _ => {}
        }
        Ok(())
    }

    fn is_subscribed(&self, market_id: &MarketId) -> bool {
        self.active_markets.read().unwrap().contains(market_id)
    }

    fn max_subscriptions(&self) -> usize {
        self.max_subscriptions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::domain::ScoreFactors;

    fn make_score(id: &str, composite: f64) -> MarketScore {
        MarketScore::new(
            MarketId::from(id.to_string()),
            ScoreFactors::default(),
            composite,
        )
    }

    #[tokio::test]
    async fn test_enqueue_and_expand() {
        let mgr = PrioritySubscriptionManager::new(100);

        // Register tokens for markets
        mgr.register_market_tokens(
            MarketId::from("m1"),
            vec![TokenId::from("t1a"), TokenId::from("t1b")],
        );
        mgr.register_market_tokens(
            MarketId::from("m2"),
            vec![TokenId::from("t2a"), TokenId::from("t2b")],
        );

        // Enqueue with different scores
        mgr.enqueue(vec![
            make_score("m1", 0.5),
            make_score("m2", 0.9), // Higher priority
        ]);

        assert_eq!(mgr.pending_count(), 2);
        assert_eq!(mgr.active_count(), 0);

        // Expand should pick highest priority first
        let added = mgr.expand(1).await.unwrap();
        assert_eq!(added.len(), 2); // m2's tokens
        assert!(mgr.is_subscribed(&MarketId::from("m2")));
        assert!(!mgr.is_subscribed(&MarketId::from("m1")));
    }

    #[tokio::test]
    async fn test_expand_respects_limit() {
        let mgr = PrioritySubscriptionManager::new(2);

        mgr.register_market_tokens(
            MarketId::from("m1"),
            vec![TokenId::from("t1a"), TokenId::from("t1b")],
        );
        mgr.register_market_tokens(
            MarketId::from("m2"),
            vec![TokenId::from("t2a"), TokenId::from("t2b")],
        );

        mgr.enqueue(vec![
            make_score("m1", 0.9),
            make_score("m2", 0.5),
        ]);

        // Can only add first market (2 tokens = max)
        let added = mgr.expand(10).await.unwrap();
        assert_eq!(added.len(), 2);
        assert_eq!(mgr.active_count(), 2);
    }

    #[tokio::test]
    async fn test_contract() {
        let mgr = PrioritySubscriptionManager::new(100);

        mgr.register_market_tokens(
            MarketId::from("m1"),
            vec![TokenId::from("t1")],
        );

        mgr.enqueue(vec![make_score("m1", 0.9)]);
        mgr.expand(1).await.unwrap();

        assert_eq!(mgr.active_count(), 1);

        let removed = mgr.contract(1).await.unwrap();
        assert_eq!(removed.len(), 1);
        assert_eq!(mgr.active_count(), 0);
    }
}
```

**Step 2: Export from service module**

Add to `src/core/service/mod.rs`:
```rust
mod priority_subscription;
pub use priority_subscription::PrioritySubscriptionManager;
```

**Step 3: Run tests**

Run: `cargo test priority_subscription`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/core/service/priority_subscription.rs src/core/service/mod.rs
git commit -m "feat(service): implement PrioritySubscriptionManager"
```

---

### Task 5.2: Implement LatencyGovernor

**Files:**
- Create: `src/core/service/latency_governor.rs`
- Modify: `src/core/service/mod.rs`

**Step 1: Create latency governor implementation**

```rust
// src/core/service/latency_governor.rs
//! Latency-based adaptive governor implementation.

use std::collections::VecDeque;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use crate::core::domain::{ResourceBudget, ScalingRecommendation};
use crate::core::service::{AdaptiveGovernor, GovernorConfig, LatencyMetrics};

/// Latency-based adaptive governor.
pub struct LatencyGovernor {
    config: GovernorConfig,
    /// Recent latency samples.
    samples: RwLock<VecDeque<Duration>>,
    /// Recent throughput samples.
    throughput: RwLock<VecDeque<f64>>,
    /// Last scaling action time.
    last_scaled: RwLock<Option<Instant>>,
    /// Resource budget.
    budget: RwLock<ResourceBudget>,
    /// Maximum samples to keep.
    max_samples: usize,
}

impl LatencyGovernor {
    /// Create a new latency governor.
    #[must_use]
    pub fn new(config: GovernorConfig) -> Self {
        Self {
            config,
            samples: RwLock::new(VecDeque::new()),
            throughput: RwLock::new(VecDeque::new()),
            last_scaled: RwLock::new(None),
            budget: RwLock::new(ResourceBudget::default()),
            max_samples: 1000,
        }
    }

    /// Calculate percentile from samples.
    fn percentile(samples: &[Duration], p: f64) -> Duration {
        if samples.is_empty() {
            return Duration::ZERO;
        }
        let mut sorted: Vec<_> = samples.iter().copied().collect();
        sorted.sort();
        let idx = ((sorted.len() as f64 * p / 100.0) as usize).min(sorted.len() - 1);
        sorted[idx]
    }

    /// Check if we're in cooldown period.
    fn in_cooldown(&self) -> bool {
        let last = self.last_scaled.read().unwrap();
        if let Some(last_time) = *last {
            Instant::now().duration_since(last_time) < self.config.scaling.cooldown
        } else {
            false
        }
    }
}

impl AdaptiveGovernor for LatencyGovernor {
    fn record_latency(&self, latency: Duration) {
        let mut samples = self.samples.write().unwrap();
        samples.push_back(latency);
        if samples.len() > self.max_samples {
            samples.pop_front();
        }
    }

    fn record_throughput(&self, messages_per_sec: f64) {
        let mut throughput = self.throughput.write().unwrap();
        throughput.push_back(messages_per_sec);
        if throughput.len() > 100 {
            throughput.pop_front();
        }
    }

    fn latency_metrics(&self) -> LatencyMetrics {
        let samples = self.samples.read().unwrap();
        let sample_vec: Vec<_> = samples.iter().copied().collect();

        LatencyMetrics {
            p50: Self::percentile(&sample_vec, 50.0),
            p95: Self::percentile(&sample_vec, 95.0),
            p99: Self::percentile(&sample_vec, 99.0),
            sample_count: sample_vec.len(),
        }
    }

    fn recommendation(&self) -> ScalingRecommendation {
        if !self.config.enabled {
            return ScalingRecommendation::Hold;
        }

        if self.in_cooldown() {
            return ScalingRecommendation::Hold;
        }

        let metrics = self.latency_metrics();
        if metrics.sample_count < 10 {
            return ScalingRecommendation::Hold;
        }

        let target_p95 = self.config.latency.target_p95;
        let expand_threshold = target_p95.mul_f64(self.config.scaling.expand_threshold);
        let contract_threshold = target_p95.mul_f64(self.config.scaling.contract_threshold);

        if metrics.p95 < expand_threshold {
            ScalingRecommendation::expand(self.config.scaling.expand_step)
        } else if metrics.p95 > contract_threshold {
            ScalingRecommendation::contract(self.config.scaling.contract_step)
        } else {
            ScalingRecommendation::Hold
        }
    }

    fn notify_scaled(&self) {
        let mut last = self.last_scaled.write().unwrap();
        *last = Some(Instant::now());
    }

    fn set_resource_budget(&self, budget: ResourceBudget) {
        let mut b = self.budget.write().unwrap();
        *b = budget;
    }

    fn config(&self) -> &GovernorConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> GovernorConfig {
        GovernorConfig {
            enabled: true,
            latency: crate::core::service::LatencyTargets {
                target_p50: Duration::from_millis(10),
                target_p95: Duration::from_millis(50),
                target_p99: Duration::from_millis(100),
                max_p99: Duration::from_millis(200),
            },
            scaling: crate::core::service::ScalingConfig {
                check_interval: Duration::from_secs(1),
                expand_threshold: 0.70,
                contract_threshold: 1.20,
                expand_step: 10,
                contract_step: 20,
                cooldown: Duration::from_millis(100),
                hysteresis: Duration::from_millis(50),
            },
        }
    }

    #[test]
    fn test_record_and_metrics() {
        let gov = LatencyGovernor::new(test_config());

        for i in 0..100 {
            gov.record_latency(Duration::from_millis(i));
        }

        let metrics = gov.latency_metrics();
        assert_eq!(metrics.sample_count, 100);
        assert!(metrics.p50 >= Duration::from_millis(40));
        assert!(metrics.p50 <= Duration::from_millis(60));
    }

    #[test]
    fn test_expand_when_low_latency() {
        let gov = LatencyGovernor::new(test_config());

        // Record low latencies (well below target)
        for _ in 0..100 {
            gov.record_latency(Duration::from_millis(10));
        }

        let rec = gov.recommendation();
        assert!(rec.is_expand());
    }

    #[test]
    fn test_contract_when_high_latency() {
        let gov = LatencyGovernor::new(test_config());

        // Record high latencies (above threshold)
        for _ in 0..100 {
            gov.record_latency(Duration::from_millis(100));
        }

        let rec = gov.recommendation();
        assert!(rec.is_contract());
    }

    #[test]
    fn test_hold_when_in_range() {
        let gov = LatencyGovernor::new(test_config());

        // Record latencies in the target range
        for _ in 0..100 {
            gov.record_latency(Duration::from_millis(45));
        }

        let rec = gov.recommendation();
        assert!(rec.is_hold());
    }

    #[test]
    fn test_cooldown_respected() {
        let gov = LatencyGovernor::new(test_config());

        for _ in 0..100 {
            gov.record_latency(Duration::from_millis(10));
        }

        // First recommendation should expand
        let rec1 = gov.recommendation();
        assert!(rec1.is_expand());

        // Notify scaled
        gov.notify_scaled();

        // During cooldown, should hold
        let rec2 = gov.recommendation();
        assert!(rec2.is_hold());
    }

    #[test]
    fn test_disabled_always_holds() {
        let mut config = test_config();
        config.enabled = false;
        let gov = LatencyGovernor::new(config);

        for _ in 0..100 {
            gov.record_latency(Duration::from_millis(10));
        }

        let rec = gov.recommendation();
        assert!(rec.is_hold());
    }
}
```

**Step 2: Export from service module**

Add to `src/core/service/mod.rs`:
```rust
mod latency_governor;
pub use latency_governor::LatencyGovernor;
```

**Step 3: Run tests**

Run: `cargo test latency_governor`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/core/service/latency_governor.rs src/core/service/mod.rs
git commit -m "feat(service): implement LatencyGovernor"
```

---

## Phase 6: Integration

### Task 6.1: Update ExchangeFactory

**Files:**
- Modify: `src/core/exchange/factory.rs`

**Step 1: Read current factory implementation**

Read the file first to understand the current structure.

**Step 2: Add factory methods for new components**

Add methods to create scorer, filter, and deduplicator:

```rust
/// Create a market scorer for the configured exchange.
pub fn create_scorer(config: &Config) -> Box<dyn MarketScorer> {
    match config.exchange {
        Exchange::Polymarket => {
            let poly_config = config.polymarket_config().unwrap();
            Box::new(PolymarketScorer::new(&poly_config.scoring))
        }
    }
}

/// Create a market filter for the configured exchange.
pub fn create_filter(config: &Config) -> Box<dyn MarketFilter> {
    match config.exchange {
        Exchange::Polymarket => {
            let poly_config = config.polymarket_config().unwrap();
            Box::new(PolymarketFilter::new(&poly_config.market_filter))
        }
    }
}

/// Create a message deduplicator for the configured exchange.
pub fn create_deduplicator(config: &Config) -> Box<dyn MessageDeduplicator> {
    match config.exchange {
        Exchange::Polymarket => {
            let poly_config = config.polymarket_config().unwrap();
            Box::new(PolymarketDeduplicator::new(&poly_config.dedup))
        }
    }
}
```

**Step 3: Add necessary imports**

Add to the imports:
```rust
use super::{MarketFilter, MarketScorer, MessageDeduplicator};
use super::polymarket::{PolymarketDeduplicator, PolymarketFilter, PolymarketScorer};
```

**Step 4: Run tests**

Run: `cargo build`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add src/core/exchange/factory.rs
git commit -m "feat(exchange): add factory methods for new components"
```

---

### Task 6.2: Add Integration Test

**Files:**
- Create: `tests/subscription_tests.rs`

**Step 1: Create integration test**

```rust
//! Integration tests for subscription management.

use edgelord::app::config::{Config, PolymarketFilterConfig, PolymarketScoringConfig};
use edgelord::core::domain::{MarketId, MarketScore, ScoreFactors, TokenId};
use edgelord::core::exchange::polymarket::{PolymarketFilter, PolymarketScorer};
use edgelord::core::exchange::{MarketFilter, MarketInfo, OutcomeInfo};
use edgelord::core::service::{PrioritySubscriptionManager, SubscriptionManager};

fn make_market(id: &str, outcomes: usize) -> MarketInfo {
    let outcome_infos: Vec<_> = (0..outcomes)
        .map(|i| OutcomeInfo {
            token_id: format!("{id}-token-{i}"),
            name: if i == 0 { "Yes".to_string() } else { "No".to_string() },
        })
        .collect();

    MarketInfo {
        id: id.to_string(),
        question: format!("Question for {id}?"),
        outcomes: outcome_infos,
        active: true,
    }
}

#[tokio::test]
async fn test_filter_score_subscribe_flow() {
    // Create components
    let filter_config = PolymarketFilterConfig::default();
    let filter = PolymarketFilter::new(&filter_config);

    let scoring_config = PolymarketScoringConfig::default();
    let scorer = PolymarketScorer::new(&scoring_config);

    let manager = PrioritySubscriptionManager::new(100);

    // Create test markets
    let markets: Vec<_> = (0..10)
        .map(|i| make_market(&format!("market-{i}"), 2))
        .collect();

    // Filter markets
    let eligible = filter.filter(&markets);
    assert_eq!(eligible.len(), 10);

    // Score markets
    let mut scores = Vec::new();
    for market in &eligible {
        let score = scorer.score(market).await.unwrap();

        // Register token mapping
        let tokens: Vec<_> = market
            .outcomes
            .iter()
            .map(|o| TokenId::from(o.token_id.clone()))
            .collect();
        manager.register_market_tokens(MarketId::from(market.id.clone()), tokens);

        scores.push(score);
    }

    // Enqueue and expand
    manager.enqueue(scores);
    assert_eq!(manager.pending_count(), 10);

    let added = manager.expand(5).await.unwrap();
    assert_eq!(added.len(), 10); // 5 markets * 2 tokens
    assert_eq!(manager.active_count(), 10);
    assert_eq!(manager.pending_count(), 5);
}

#[tokio::test]
async fn test_subscription_limits_respected() {
    let manager = PrioritySubscriptionManager::new(4); // Only 4 tokens allowed

    // Register 3 markets with 2 tokens each
    for i in 0..3 {
        manager.register_market_tokens(
            MarketId::from(format!("m{i}")),
            vec![
                TokenId::from(format!("m{i}-yes")),
                TokenId::from(format!("m{i}-no")),
            ],
        );
    }

    let scores: Vec<_> = (0..3)
        .map(|i| {
            MarketScore::new(
                MarketId::from(format!("m{i}")),
                ScoreFactors::default(),
                1.0 - (i as f64 * 0.1), // Decreasing priority
            )
        })
        .collect();

    manager.enqueue(scores);

    // Try to expand all 3, but limit is 4 tokens
    let added = manager.expand(10).await.unwrap();
    assert_eq!(added.len(), 4); // Only 2 markets fit (4 tokens)
    assert_eq!(manager.active_count(), 4);
}
```

**Step 2: Run tests**

Run: `cargo test subscription_tests`
Expected: All tests pass

**Step 3: Commit**

```bash
git add tests/subscription_tests.rs
git commit -m "test: add subscription management integration tests"
```

---

## Phase 7: Documentation

### Task 7.1: Update Example Config

**Files:**
- Modify: `config.example.toml` (or create if doesn't exist)

**Step 1: Add comprehensive example configuration**

Create/update with all new sections documented.

**Step 2: Commit**

```bash
git add config.example.toml
git commit -m "docs: update example config with subscription management options"
```

---

## Summary

This implementation plan covers:

1. **Phase 1**: Core domain types (ScoreFactors, MarketScore, ResourceBudget, ScalingRecommendation)
2. **Phase 2**: Trait abstractions (MarketScorer, MarketFilter, MessageDeduplicator, AdaptiveGovernor, SubscriptionManager)
3. **Phase 3**: Configuration schema (profiles, resources, governor, connection pool, exchange-specific)
4. **Phase 4**: Polymarket implementations (scorer, filter, deduplicator)
5. **Phase 5**: Core implementations (PrioritySubscriptionManager, LatencyGovernor)
6. **Phase 6**: Integration (ExchangeFactory updates, integration tests)
7. **Phase 7**: Documentation

**Not included in this plan (future work):**
- ConnectionPool with sharding and redundancy
- Orchestrator integration
- Runtime adaptive scaling loop
- E2E tests with real Polymarket connection
