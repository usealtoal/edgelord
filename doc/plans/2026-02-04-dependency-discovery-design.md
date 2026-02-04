# Dependency Discovery Service Design

> **Date:** 2026-02-04
> **Status:** Draft
> **Author:** Subagent (dependency-discovery-design)
> **Tracking:** Branch `chud/dependency-discovery`

## Executive Summary

This document designs the LLM-powered Dependency Discovery Service for edgelord's combinatorial arbitrage strategy. The service discovers logical relationships between prediction markets (e.g., "Trump wins PA" implies contribution to "Trump wins nationally") and provides pre-computed constraints to the Frank-Wolfe solver.

**Key Design Principles:**
1. **Event-driven** — Analyze on market creation and significant price changes
2. **Two-tier architecture** — LLM discovers (slow path), solver executes (hot path)
3. **Trait-based & pluggable** — Multiple discovery backends (LLM, rules, hybrid)
4. **Fail-safe** — Bad LLM output cannot cause incorrect trades

---

## Problem Statement

The combinatorial arbitrage infrastructure (Frank-Wolfe + HiGHS) is implemented and tested, but `CombinatorialStrategy::detect()` returns empty because:

1. No way to identify which markets are logically related
2. No way to encode those relationships as ILP constraints
3. `MarketContext.has_dependencies` is always `false`

**Goal:** Build a service that automatically discovers market dependencies, caches them, and feeds constraint matrices to the solver.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Event Sources                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐   │
│  │ New Market   │  │ Price Change │  │ Periodic Full Scan   │   │
│  │ Detected     │  │ > Threshold  │  │ (hourly)             │   │
│  └──────┬───────┘  └──────┬───────┘  └──────────┬───────────┘   │
└─────────┼─────────────────┼─────────────────────┼───────────────┘
          │                 │                     │
          v                 v                     v
┌─────────────────────────────────────────────────────────────────┐
│                  DependencyDiscoveryService                      │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │ Event Queue (bounded, deduplicated)                     │    │
│  └─────────────────────────────┬───────────────────────────┘    │
│                                │                                 │
│  ┌─────────────────────────────v───────────────────────────┐    │
│  │ Discovery Coordinator                                    │    │
│  │  - Batches markets for analysis                          │    │
│  │  - Manages discovery rate limits                         │    │
│  │  - Routes to appropriate discoverer                      │    │
│  └─────────────────────────────┬───────────────────────────┘    │
│                                │                                 │
│  ┌─────────────────────────────v───────────────────────────┐    │
│  │ DependencyDiscoverer (trait)                             │    │
│  │  ├── LlmDiscoverer (Claude, GPT-4, etc.)                 │    │
│  │  ├── RuleBasedDiscoverer (heuristics)                    │    │
│  │  └── HybridDiscoverer (rules + LLM validation)           │    │
│  └─────────────────────────────┬───────────────────────────┘    │
│                                │                                 │
│  ┌─────────────────────────────v───────────────────────────┐    │
│  │ Validation Layer                                         │    │
│  │  - Schema validation (parseable JSON)                    │    │
│  │  - Semantic validation (markets exist, constraints sane) │    │
│  │  - Confidence filtering (reject low-confidence)          │    │
│  └─────────────────────────────┬───────────────────────────┘    │
│                                │                                 │
│  ┌─────────────────────────────v───────────────────────────┐    │
│  │ DependencyCache                                          │    │
│  │  - Stores validated dependencies with TTL                │    │
│  │  - Indexed by market cluster                             │    │
│  │  - Pre-computed ILP constraint matrices                  │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
                                │
                                v
┌─────────────────────────────────────────────────────────────────┐
│                  CombinatorialStrategy                           │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │ detect() - HOT PATH                                      │    │
│  │  1. Get cluster for market from cache                    │    │
│  │  2. Get pre-computed constraints                         │    │
│  │  3. Run Frank-Wolfe projection                           │    │
│  │  4. Return opportunities if gap > threshold              │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

---

## Core Domain Types

### Dependencies

```rust
// Location: src/core/domain/dependency.rs

/// A logical dependency between prediction markets.
#[derive(Debug, Clone, PartialEq)]
pub struct Dependency {
    /// Unique identifier for this dependency.
    pub id: DependencyId,
    /// The type and semantics of the dependency.
    pub kind: DependencyKind,
    /// Confidence score (0.0 - 1.0) from discoverer.
    pub confidence: f64,
    /// Human-readable reasoning (for debugging/audit).
    pub reasoning: String,
    /// When this dependency was discovered.
    pub discovered_at: chrono::DateTime<chrono::Utc>,
    /// When this dependency expires (needs re-validation).
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// The type of logical relationship between markets.
#[derive(Debug, Clone, PartialEq)]
pub enum DependencyKind {
    /// If market A resolves YES, market B must resolve YES.
    /// Constraint: P(A) ≤ P(B), encoded as μ_A - μ_B ≤ 0
    Implies {
        if_yes: MarketId,
        then_yes: MarketId,
    },

    /// At most one of these markets can resolve YES.
    /// Constraint: Σ μ_i ≤ 1
    MutuallyExclusive(Vec<MarketId>),

    /// Exactly one of these markets must resolve YES.
    /// Constraint: Σ μ_i = 1
    ExactlyOne(Vec<MarketId>),

    /// Custom linear constraint: Σ (coeff_i × μ_i) {≤, =, ≥} rhs
    LinearConstraint {
        terms: Vec<(MarketId, Decimal)>,
        sense: ConstraintSense,
        rhs: Decimal,
    },
}

impl DependencyKind {
    /// Convert to ILP constraint(s) for the solver.
    pub fn to_constraints(&self, market_indices: &HashMap<MarketId, usize>) -> Vec<Constraint> {
        match self {
            Self::Implies { if_yes, then_yes } => {
                // μ_A - μ_B ≤ 0  =>  μ_A ≤ μ_B
                let mut coeffs = vec![Decimal::ZERO; market_indices.len()];
                coeffs[market_indices[if_yes]] = Decimal::ONE;
                coeffs[market_indices[then_yes]] = -Decimal::ONE;
                vec![Constraint::leq(coeffs, Decimal::ZERO)]
            }
            Self::MutuallyExclusive(markets) => {
                let mut coeffs = vec![Decimal::ZERO; market_indices.len()];
                for m in markets {
                    coeffs[market_indices[m]] = Decimal::ONE;
                }
                vec![Constraint::leq(coeffs, Decimal::ONE)]
            }
            Self::ExactlyOne(markets) => {
                let mut coeffs = vec![Decimal::ZERO; market_indices.len()];
                for m in markets {
                    coeffs[market_indices[m]] = Decimal::ONE;
                }
                vec![Constraint::eq(coeffs, Decimal::ONE)]
            }
            Self::LinearConstraint { terms, sense, rhs } => {
                let mut coeffs = vec![Decimal::ZERO; market_indices.len()];
                for (market_id, coeff) in terms {
                    coeffs[market_indices[market_id]] = *coeff;
                }
                vec![match sense {
                    ConstraintSense::LessEqual => Constraint::leq(coeffs, *rhs),
                    ConstraintSense::GreaterEqual => Constraint::geq(coeffs, *rhs),
                    ConstraintSense::Equal => Constraint::eq(coeffs, *rhs),
                }]
            }
        }
    }

    /// Get all markets referenced by this dependency.
    pub fn markets(&self) -> Vec<&MarketId> {
        match self {
            Self::Implies { if_yes, then_yes } => vec![if_yes, then_yes],
            Self::MutuallyExclusive(ms) | Self::ExactlyOne(ms) => ms.iter().collect(),
            Self::LinearConstraint { terms, .. } => terms.iter().map(|(m, _)| m).collect(),
        }
    }
}

/// A cluster of related markets with pre-computed constraints.
#[derive(Debug, Clone)]
pub struct MarketCluster {
    /// Unique identifier for this cluster.
    pub id: ClusterId,
    /// Markets in this cluster (ordered for ILP variable mapping).
    pub markets: Vec<MarketId>,
    /// Dependencies within this cluster.
    pub dependencies: Vec<Dependency>,
    /// Pre-computed ILP constraint matrix.
    pub constraints: Vec<Constraint>,
    /// Last update timestamp.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl MarketCluster {
    /// Build ILP problem for this cluster given current prices.
    pub fn build_ilp(&self, prices: &HashMap<MarketId, Decimal>) -> IlpProblem {
        let num_vars = self.markets.len();
        let mut lp = LpProblem::new(num_vars);
        
        // Set bounds [0, 1] for all probability variables
        lp.bounds = vec![VariableBounds::binary(); num_vars];
        
        // Add pre-computed constraints
        lp.constraints = self.constraints.clone();
        
        IlpProblem::new(lp, vec![]) // LP relaxation for Frank-Wolfe
    }
}
```

### Newtype IDs

```rust
// Location: src/core/domain/id.rs (extend existing)

/// Unique identifier for a dependency.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DependencyId(String);

impl DependencyId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

/// Unique identifier for a market cluster.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClusterId(String);
```

---

## Service Layer

### DependencyDiscoverer Trait

```rust
// Location: src/core/service/discovery/mod.rs

use async_trait::async_trait;
use crate::core::domain::{Dependency, MarketId};
use crate::core::exchange::MarketInfo;
use crate::error::Result;

/// Discovers logical dependencies between markets.
///
/// Implementations may use LLMs, heuristic rules, or hybrid approaches.
/// All discoverers must be idempotent and safe to call repeatedly.
#[async_trait]
pub trait DependencyDiscoverer: Send + Sync {
    /// Discoverer name for logging and config.
    fn name(&self) -> &'static str;

    /// Discover dependencies among a set of markets.
    ///
    /// # Arguments
    /// * `markets` - Market metadata to analyze
    /// * `existing` - Already-known dependencies (for incremental discovery)
    ///
    /// # Returns
    /// New dependencies discovered. May overlap with `existing` (caller dedupes).
    async fn discover(
        &self,
        markets: &[MarketInfo],
        existing: &[Dependency],
    ) -> Result<Vec<Dependency>>;

    /// Check if this discoverer can handle the given market count.
    ///
    /// LLM discoverers may have context window limits.
    fn max_markets(&self) -> usize {
        100 // Conservative default
    }
}

/// Configuration for the discovery service.
#[derive(Debug, Clone, Deserialize)]
pub struct DiscoveryConfig {
    /// Minimum confidence to accept a dependency.
    #[serde(default = "default_min_confidence")]
    pub min_confidence: f64,

    /// How long discovered dependencies are valid (seconds).
    #[serde(default = "default_ttl_seconds")]
    pub ttl_seconds: u64,

    /// Price change threshold to trigger re-analysis (0.0 - 1.0).
    #[serde(default = "default_price_threshold")]
    pub price_change_threshold: f64,

    /// Maximum pending discovery requests in queue.
    #[serde(default = "default_queue_size")]
    pub max_queue_size: usize,

    /// Batch size for LLM analysis.
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    /// Rate limit: max discoveries per minute.
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_minute: u32,
}

fn default_min_confidence() -> f64 { 0.7 }
fn default_ttl_seconds() -> u64 { 3600 } // 1 hour
fn default_price_threshold() -> f64 { 0.05 } // 5%
fn default_queue_size() -> usize { 1000 }
fn default_batch_size() -> usize { 20 }
fn default_rate_limit() -> u32 { 30 }
```

### LLM Discoverer

```rust
// Location: src/core/service/discovery/llm.rs

/// LLM-powered dependency discoverer.
pub struct LlmDiscoverer {
    client: LlmClient,
    config: LlmDiscoveryConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmDiscoveryConfig {
    /// LLM provider (claude, openai, local).
    pub provider: String,
    /// Model name (claude-3-sonnet, gpt-4-turbo, etc.).
    pub model: String,
    /// API endpoint (optional, for local models).
    pub endpoint: Option<String>,
    /// Temperature for generation.
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    /// Maximum tokens in response.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
}

fn default_temperature() -> f64 { 0.3 } // Low for consistency
fn default_max_tokens() -> usize { 4096 }

#[async_trait]
impl DependencyDiscoverer for LlmDiscoverer {
    fn name(&self) -> &'static str {
        "llm"
    }

    async fn discover(
        &self,
        markets: &[MarketInfo],
        _existing: &[Dependency],
    ) -> Result<Vec<Dependency>> {
        let prompt = self.build_prompt(markets);
        let response = self.client.complete(&prompt).await?;
        self.parse_response(&response, markets)
    }

    fn max_markets(&self) -> usize {
        // Context window considerations
        50 // ~200 tokens per market description
    }
}

impl LlmDiscoverer {
    fn build_prompt(&self, markets: &[MarketInfo]) -> String {
        let market_list = markets
            .iter()
            .enumerate()
            .map(|(i, m)| format!("{}. [{}] \"{}\"", i + 1, m.id, m.question))
            .collect::<Vec<_>>()
            .join("\n");

        format!(r#"Analyze these prediction markets for logical dependencies.

## Markets
{market_list}

## Task
Identify logical relationships where the outcome of one market constrains another.

## Dependency Types
1. **implies**: If market A resolves YES, market B must resolve YES.
   Example: "Biden wins Wisconsin" implies "Biden wins at least one swing state"

2. **mutually_exclusive**: At most one of these markets can resolve YES.
   Example: "Trump wins" and "Biden wins" for same election

3. **exactly_one**: Exactly one of these markets must resolve YES.
   Example: All candidates in a single-winner election

4. **linear_constraint**: Custom mathematical relationship.
   Example: P(A) + P(B) ≤ 1.5 for correlated events

## Output Format (JSON)
```json
{{
  "dependencies": [
    {{
      "type": "implies",
      "if_yes": "market_id_1",
      "then_yes": "market_id_2",
      "confidence": 0.95,
      "reasoning": "Brief explanation"
    }},
    {{
      "type": "mutually_exclusive",
      "markets": ["market_id_1", "market_id_2"],
      "confidence": 0.99,
      "reasoning": "Brief explanation"
    }}
  ]
}}
```

## Rules
- Only output high-confidence dependencies (>0.7)
- Use market IDs exactly as provided
- Provide brief, clear reasoning
- If no dependencies exist, return {{"dependencies": []}}
- Do NOT invent markets or IDs
"#)
    }

    fn parse_response(
        &self,
        response: &str,
        markets: &[MarketInfo],
    ) -> Result<Vec<Dependency>> {
        // Extract JSON from response (handle markdown code blocks)
        let json_str = self.extract_json(response)?;
        let parsed: LlmResponse = serde_json::from_str(&json_str)?;
        
        // Validate and convert
        let market_ids: HashSet<_> = markets.iter().map(|m| &m.id).collect();
        let mut dependencies = Vec::new();
        
        for dep in parsed.dependencies {
            // Validate all referenced markets exist
            if !self.validate_market_refs(&dep, &market_ids) {
                tracing::warn!(dep = ?dep, "LLM referenced unknown market, skipping");
                continue;
            }
            
            if let Some(valid_dep) = self.convert_dependency(dep) {
                dependencies.push(valid_dep);
            }
        }
        
        Ok(dependencies)
    }
}
```

### Dependency Cache

```rust
// Location: src/core/cache/dependency.rs

use std::collections::HashMap;
use std::sync::RwLock;
use chrono::{DateTime, Utc};

use crate::core::domain::{ClusterId, Dependency, MarketCluster, MarketId};

/// Cache for discovered dependencies and market clusters.
///
/// Thread-safe, supports TTL-based expiration.
pub struct DependencyCache {
    /// Dependencies indexed by involved market.
    by_market: RwLock<HashMap<MarketId, Vec<Dependency>>>,
    /// Pre-computed clusters.
    clusters: RwLock<HashMap<ClusterId, MarketCluster>>,
    /// Market → Cluster mapping.
    market_to_cluster: RwLock<HashMap<MarketId, ClusterId>>,
    /// Configuration.
    config: CacheConfig,
}

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub default_ttl: chrono::Duration,
    pub max_dependencies: usize,
    pub max_clusters: usize,
}

impl DependencyCache {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            by_market: RwLock::new(HashMap::new()),
            clusters: RwLock::new(HashMap::new()),
            market_to_cluster: RwLock::new(HashMap::new()),
            config,
        }
    }

    /// Get cluster for a market (if any dependencies exist).
    pub fn get_cluster(&self, market_id: &MarketId) -> Option<MarketCluster> {
        let mapping = self.market_to_cluster.read().ok()?;
        let cluster_id = mapping.get(market_id)?;
        
        let clusters = self.clusters.read().ok()?;
        let cluster = clusters.get(cluster_id)?;
        
        // Check expiration
        if cluster.updated_at + self.config.default_ttl < Utc::now() {
            return None; // Expired
        }
        
        Some(cluster.clone())
    }

    /// Add new dependencies and rebuild affected clusters.
    pub fn add_dependencies(&self, dependencies: Vec<Dependency>) {
        if dependencies.is_empty() {
            return;
        }

        // Group by cluster (using union-find for connected components)
        let clusters = self.build_clusters(&dependencies);
        
        // Update cache atomically
        let mut by_market = self.by_market.write().unwrap();
        let mut cluster_cache = self.clusters.write().unwrap();
        let mut mapping = self.market_to_cluster.write().unwrap();
        
        for dep in &dependencies {
            for market_id in dep.kind.markets() {
                by_market
                    .entry(market_id.clone())
                    .or_default()
                    .push(dep.clone());
            }
        }
        
        for cluster in clusters {
            for market_id in &cluster.markets {
                mapping.insert(market_id.clone(), cluster.id.clone());
            }
            cluster_cache.insert(cluster.id.clone(), cluster);
        }
    }

    /// Invalidate dependencies involving a market.
    pub fn invalidate(&self, market_id: &MarketId) {
        let mut by_market = self.by_market.write().unwrap();
        by_market.remove(market_id);
        
        // Also invalidate cluster
        let mut mapping = self.market_to_cluster.write().unwrap();
        if let Some(cluster_id) = mapping.remove(market_id) {
            let mut clusters = self.clusters.write().unwrap();
            clusters.remove(&cluster_id);
        }
    }

    /// Build clusters from dependencies using union-find.
    fn build_clusters(&self, dependencies: &[Dependency]) -> Vec<MarketCluster> {
        // Union-find to group connected markets
        let mut uf = UnionFind::new();
        
        for dep in dependencies {
            let markets: Vec<_> = dep.kind.markets().into_iter().cloned().collect();
            for window in markets.windows(2) {
                uf.union(&window[0], &window[1]);
            }
        }
        
        // Group dependencies by cluster root
        let mut cluster_deps: HashMap<MarketId, Vec<Dependency>> = HashMap::new();
        for dep in dependencies {
            let root = uf.find(dep.kind.markets()[0]);
            cluster_deps.entry(root.clone()).or_default().push(dep.clone());
        }
        
        // Build MarketCluster for each group
        cluster_deps
            .into_iter()
            .map(|(_, deps)| self.build_single_cluster(deps))
            .collect()
    }

    fn build_single_cluster(&self, dependencies: Vec<Dependency>) -> MarketCluster {
        // Collect all unique markets
        let mut markets: Vec<MarketId> = dependencies
            .iter()
            .flat_map(|d| d.kind.markets().into_iter().cloned())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        markets.sort(); // Deterministic ordering
        
        // Build index mapping
        let market_indices: HashMap<MarketId, usize> = markets
            .iter()
            .enumerate()
            .map(|(i, m)| (m.clone(), i))
            .collect();
        
        // Convert dependencies to constraints
        let constraints: Vec<_> = dependencies
            .iter()
            .flat_map(|d| d.kind.to_constraints(&market_indices))
            .collect();
        
        MarketCluster {
            id: ClusterId::new(),
            markets,
            dependencies,
            constraints,
            updated_at: Utc::now(),
        }
    }
}
```

### Discovery Service

```rust
// Location: src/core/service/discovery/service.rs

use std::sync::Arc;
use tokio::sync::mpsc;

/// Event that triggers dependency discovery.
#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
    /// New market detected.
    NewMarket(MarketInfo),
    /// Significant price change on existing market.
    PriceChange {
        market_id: MarketId,
        old_price: Decimal,
        new_price: Decimal,
    },
    /// Periodic full scan request.
    FullScan,
}

/// The main dependency discovery service.
pub struct DependencyDiscoveryService {
    /// The discoverer implementation.
    discoverer: Arc<dyn DependencyDiscoverer>,
    /// Cache for results.
    cache: Arc<DependencyCache>,
    /// Configuration.
    config: DiscoveryConfig,
    /// Event receiver.
    event_rx: mpsc::Receiver<DiscoveryEvent>,
    /// Event sender (cloned to event sources).
    event_tx: mpsc::Sender<DiscoveryEvent>,
    /// All known markets for batch analysis.
    known_markets: RwLock<Vec<MarketInfo>>,
}

impl DependencyDiscoveryService {
    pub fn new(
        discoverer: Arc<dyn DependencyDiscoverer>,
        cache: Arc<DependencyCache>,
        config: DiscoveryConfig,
    ) -> Self {
        let (event_tx, event_rx) = mpsc::channel(config.max_queue_size);
        Self {
            discoverer,
            cache,
            config,
            event_rx,
            event_tx,
            known_markets: RwLock::new(Vec::new()),
        }
    }

    /// Get a sender for discovery events.
    pub fn event_sender(&self) -> mpsc::Sender<DiscoveryEvent> {
        self.event_tx.clone()
    }

    /// Run the discovery service (call from orchestrator).
    pub async fn run(&mut self) {
        let mut rate_limiter = RateLimiter::new(self.config.rate_limit_per_minute);
        let mut batch: Vec<MarketInfo> = Vec::new();
        let mut batch_timeout = tokio::time::interval(Duration::from_secs(5));

        loop {
            tokio::select! {
                Some(event) = self.event_rx.recv() => {
                    match event {
                        DiscoveryEvent::NewMarket(info) => {
                            self.known_markets.write().unwrap().push(info.clone());
                            batch.push(info);
                        }
                        DiscoveryEvent::PriceChange { market_id, .. } => {
                            // Invalidate and re-queue for discovery
                            self.cache.invalidate(&MarketId::from(market_id.as_str()));
                            if let Some(info) = self.get_market_info(&market_id) {
                                batch.push(info);
                            }
                        }
                        DiscoveryEvent::FullScan => {
                            batch = self.known_markets.read().unwrap().clone();
                        }
                    }
                }
                _ = batch_timeout.tick() => {
                    if !batch.is_empty() && rate_limiter.check() {
                        self.process_batch(&mut batch).await;
                    }
                }
            }

            // Process batch if it reaches threshold
            if batch.len() >= self.config.batch_size && rate_limiter.check() {
                self.process_batch(&mut batch).await;
            }
        }
    }

    async fn process_batch(&self, batch: &mut Vec<MarketInfo>) {
        if batch.is_empty() {
            return;
        }

        let markets: Vec<_> = batch.drain(..).collect();
        let existing = self.get_existing_dependencies(&markets);

        match self.discoverer.discover(&markets, &existing).await {
            Ok(dependencies) => {
                // Filter by confidence
                let valid: Vec<_> = dependencies
                    .into_iter()
                    .filter(|d| d.confidence >= self.config.min_confidence)
                    .collect();

                if !valid.is_empty() {
                    tracing::info!(
                        count = valid.len(),
                        "Discovered new dependencies"
                    );
                    self.cache.add_dependencies(valid);
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "Discovery failed, will retry");
                // Re-queue for retry (with backoff handled by rate limiter)
            }
        }
    }
}
```

---

## Integration with CombinatorialStrategy

### Updated Strategy Implementation

```rust
// Location: src/core/strategy/combinatorial/mod.rs (modifications)

impl Strategy for CombinatorialStrategy {
    fn name(&self) -> &'static str {
        "combinatorial"
    }

    fn applies_to(&self, ctx: &MarketContext) -> bool {
        // Strategy now checks cache dynamically
        self.config.enabled && ctx.has_dependencies
    }

    fn detect(&self, ctx: &DetectionContext) -> Vec<Opportunity> {
        // Get cluster from cache
        let cluster = match self.dependency_cache.get_cluster(ctx.market.market_id()) {
            Some(c) => c,
            None => return vec![], // No known dependencies
        };

        // Gather prices for all markets in cluster
        let prices = match self.gather_cluster_prices(&cluster, ctx.cache) {
            Some(p) => p,
            None => return vec![], // Missing price data
        };

        // Build ILP and run Frank-Wolfe
        let ilp = cluster.build_ilp(&prices);
        let theta: Vec<Decimal> = cluster.markets
            .iter()
            .map(|m| prices[m])
            .collect();

        let result = match self.fw.project(&theta, &ilp, &self.solver) {
            Ok(r) => r,
            Err(e) => {
                tracing::debug!(error = %e, "Frank-Wolfe projection failed");
                return vec![];
            }
        };

        // Check if arbitrage exists
        if result.has_arbitrage(self.config.gap_threshold) {
            vec![self.build_opportunity(&cluster, &result, &prices)]
        } else {
            vec![]
        }
    }
}
```

### MarketContext Enhancement

```rust
// Location: src/core/strategy/context.rs (modifications)

impl<'a> DetectionContext<'a> {
    /// Create context with dependency cache lookup.
    pub fn new_with_dependencies(
        market: &'a Market,
        cache: &'a OrderBookCache,
        dep_cache: &'a DependencyCache,
    ) -> Self {
        let has_deps = dep_cache.get_cluster(market.market_id()).is_some();
        let correlated = if has_deps {
            dep_cache
                .get_cluster(market.market_id())
                .map(|c| c.markets.clone())
                .unwrap_or_default()
        } else {
            vec![]
        };

        let market_ctx = MarketContext {
            outcome_count: market.outcome_count(),
            has_dependencies: has_deps,
            correlated_markets: correlated,
        };

        Self {
            market,
            cache,
            market_ctx,
        }
    }
}
```

---

## Event Integration

### Price Change Detection

The orchestrator's event handler monitors for significant price changes:

```rust
// Location: src/app/orchestrator/handler.rs (additions)

/// Track prices for change detection.
struct PriceTracker {
    last_prices: HashMap<TokenId, Decimal>,
    threshold: Decimal,
    discovery_tx: mpsc::Sender<DiscoveryEvent>,
}

impl PriceTracker {
    fn check_and_update(&mut self, token_id: &TokenId, new_price: Decimal) {
        if let Some(old_price) = self.last_prices.get(token_id) {
            let change = ((new_price - old_price) / old_price).abs();
            if change > self.threshold {
                // Trigger discovery
                let _ = self.discovery_tx.try_send(DiscoveryEvent::PriceChange {
                    market_id: token_id.clone().into(), // Need market mapping
                    old_price: *old_price,
                    new_price,
                });
            }
        }
        self.last_prices.insert(token_id.clone(), new_price);
    }
}
```

### Startup Initialization

```rust
// Location: src/app/orchestrator/mod.rs (additions)

impl Orchestrator {
    pub async fn run(config: Config) -> Result<()> {
        // ... existing initialization ...

        // Initialize dependency discovery
        let dep_cache = Arc::new(DependencyCache::new(CacheConfig {
            default_ttl: chrono::Duration::seconds(config.discovery.ttl_seconds as i64),
            max_dependencies: 10_000,
            max_clusters: 1_000,
        }));

        let discoverer: Arc<dyn DependencyDiscoverer> = match &config.discovery.provider {
            "llm" => Arc::new(LlmDiscoverer::new(config.discovery.llm.clone())),
            "rules" => Arc::new(RuleBasedDiscoverer::new()),
            _ => Arc::new(NullDiscoverer),
        };

        let mut discovery_service = DependencyDiscoveryService::new(
            discoverer,
            dep_cache.clone(),
            config.discovery.clone(),
        );

        // Spawn discovery service
        let discovery_tx = discovery_service.event_sender();
        tokio::spawn(async move {
            discovery_service.run().await;
        });

        // Queue initial full scan
        discovery_tx.send(DiscoveryEvent::FullScan).await?;

        // ... rest of initialization ...
    }
}
```

---

## Configuration

### Config File Additions

```toml
# config.toml

[discovery]
# Discoverer backend: "llm", "rules", "hybrid", "null"
provider = "llm"

# Minimum confidence to accept (0.0 - 1.0)
min_confidence = 0.7

# Dependency TTL in seconds (3600 = 1 hour)
ttl_seconds = 3600

# Price change threshold to trigger re-analysis (5%)
price_change_threshold = 0.05

# Event queue size
max_queue_size = 1000

# Batch size for LLM calls
batch_size = 20

# Rate limit: max discoveries per minute
rate_limit_per_minute = 30

[discovery.llm]
# LLM provider
provider = "anthropic"

# Model name
model = "claude-3-5-sonnet-20241022"

# Temperature (lower = more consistent)
temperature = 0.3

# Max response tokens
max_tokens = 4096

# API key (from environment)
# api_key = "${ANTHROPIC_API_KEY}"
```

---

## Error Handling

### LLM Failure Modes

| Failure | Detection | Response |
|---------|-----------|----------|
| Invalid JSON | Parse error | Log, retry with backoff |
| Unknown market ID | Validation | Skip that dependency |
| Low confidence | Threshold check | Filter out |
| Timeout | Request timeout | Retry with backoff |
| Rate limit | HTTP 429 | Exponential backoff |
| Hallucinated relationship | Semantic validation | Cannot detect - rely on confidence |

### Safety Guarantees

1. **No direct trade decisions from LLM** — LLM only suggests constraints, solver validates
2. **Confidence filtering** — Only high-confidence (>0.7) dependencies used
3. **Cache TTL** — Stale dependencies expire, forcing re-validation
4. **Solver verification** — Frank-Wolfe won't produce invalid trades from bad constraints

---

## Module Structure

```
src/
├── core/
│   ├── domain/
│   │   ├── mod.rs              # Add dependency exports
│   │   └── dependency.rs       # NEW: Dependency, DependencyKind, MarketCluster
│   │
│   ├── cache/
│   │   ├── mod.rs              # Add DependencyCache export
│   │   └── dependency.rs       # NEW: DependencyCache
│   │
│   ├── service/
│   │   ├── mod.rs              # Add discovery exports
│   │   └── discovery/          # NEW: Discovery service
│   │       ├── mod.rs          # Trait + config
│   │       ├── llm.rs          # LLM discoverer
│   │       ├── rules.rs        # Rule-based discoverer
│   │       └── service.rs      # Discovery service coordinator
│   │
│   └── strategy/
│       └── combinatorial/
│           └── mod.rs          # Update detect() implementation
│
└── app/
    ├── config/
    │   ├── mod.rs              # Add DiscoveryConfig export
    │   └── discovery.rs        # NEW: Discovery config
    │
    └── orchestrator/
        ├── mod.rs              # Wire up discovery service
        └── handler.rs          # Add price change tracking
```

---

## Implementation Plan

### Phase 1: Domain Types (Day 1)
- [ ] Add `Dependency`, `DependencyKind`, `MarketCluster` to domain
- [ ] Add `DependencyId`, `ClusterId` to id.rs
- [ ] Write unit tests for constraint generation

### Phase 2: Cache Layer (Day 1-2)
- [ ] Implement `DependencyCache` with TTL support
- [ ] Implement union-find for cluster building
- [ ] Write tests for cache operations

### Phase 3: Discoverer Trait (Day 2)
- [ ] Define `DependencyDiscoverer` trait
- [ ] Implement `NullDiscoverer` (no-op for testing)
- [ ] Implement `RuleBasedDiscoverer` (simple heuristics)

### Phase 4: LLM Integration (Day 3-4)
- [ ] Add `LlmClient` abstraction
- [ ] Implement `LlmDiscoverer` with prompt engineering
- [ ] Add response parsing and validation
- [ ] Write integration tests with mock LLM

### Phase 5: Service Integration (Day 4-5)
- [ ] Implement `DependencyDiscoveryService`
- [ ] Add configuration support
- [ ] Wire into orchestrator
- [ ] Add price change tracking

### Phase 6: Strategy Update (Day 5)
- [ ] Update `CombinatorialStrategy::detect()`
- [ ] Update `DetectionContext` with dependency lookup
- [ ] End-to-end testing

### Phase 7: Polish (Day 6)
- [ ] Documentation
- [ ] Metrics/logging
- [ ] Configuration validation
- [ ] Edge case handling

---

## Open Questions

1. **LLM Provider Priority:** Claude vs GPT-4 vs local model?
   - Claude 3.5 Sonnet recommended for consistency and JSON output quality

2. **Confidence Calibration:** How to tune 0.7 threshold?
   - Start conservative, lower if missing real dependencies

3. **Full Scan Frequency:** How often to re-analyze all markets?
   - Suggest: hourly, or on significant market events (>10 new markets)

4. **Multi-Exchange Dependencies:** Cross-exchange correlations?
   - Out of scope for v1, but architecture supports it

5. **Persistence:** Should dependencies survive restarts?
   - Suggest: Yes, add simple file-based persistence

---

## References

- [Arbitrage-Free Combinatorial Market Making (arXiv:1606.02825)](https://arxiv.org/abs/1606.02825)
- [Unravelling the Probabilistic Forest (arXiv:2508.03474)](https://arxiv.org/abs/2508.03474)
- Existing: `doc/research/combinatorial-future.md`
- Existing: `src/core/strategy/combinatorial/`
