# Relation Inference Service Design

> **Date:** 2026-02-04
> **Status:** Draft
> **Author:** Subagent (constraint-inference-design)
> **Tracking:** Branch `chud/dependency-discovery`

## Executive Summary

This document designs the LLM-powered Relation Inference Service for edgelord's combinatorial arbitrage strategy. The service infers logical relationships between prediction markets (e.g., "Trump wins PA" implies contribution to "Trump wins nationally") and provides pre-computed constraints to the Frank-Wolfe solver.

**Key Design Principles:**
1. **Event-driven** — Analyze on market creation and significant price changes
2. **Two-tier architecture** — LLM infers (slow path), solver executes (hot path)
3. **Trait-based & pluggable** — Multiple inference backends (LLM, rules, hybrid)
4. **Fail-safe** — Bad LLM output cannot cause incorrect trades

---

## Problem Statement

The combinatorial arbitrage infrastructure (Frank-Wolfe + HiGHS) is implemented and tested, but `CombinatorialStrategy::detect()` returns empty because:

1. No way to identify which markets are logically related
2. No way to encode those relationships as ILP constraints
3. `MarketContext.has_relations` is always `false`

**Goal:** Build a service that automatically infers market constraints, caches them, and feeds constraint matrices to the solver.

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
│                  InferenceService                      │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │ Event Queue (bounded, deduplicated)                     │    │
│  └─────────────────────────────┬───────────────────────────┘    │
│                                │                                 │
│  ┌─────────────────────────────v───────────────────────────┐    │
│  │ Inference Coordinator                                    │    │
│  │  - Batches markets for analysis                          │    │
│  │  - Manages inference rate limits                         │    │
│  │  - Routes to appropriate inferrer                      │    │
│  └─────────────────────────────┬───────────────────────────┘    │
│                                │                                 │
│  ┌─────────────────────────────v───────────────────────────┐    │
│  │ Inferrer (trait)                             │    │
│  │  ├── LlmInferrer (Claude, GPT-4, etc.)                 │    │
│  │  ├── RuleInferrer (heuristics)                    │    │
│  │  └── HybridInferrer (rules + LLM validation)           │    │
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
│  │ ClusterCache                                          │    │
│  │  - Stores validated constraints with TTL                │    │
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

### Relations

```rust
// Location: src/core/domain/relation.rs

/// A logical relation between prediction markets.
#[derive(Debug, Clone, PartialEq)]
pub struct Relation {
    /// Unique identifier for this relation.
    pub id: RelationId,
    /// The type and semantics of the relation.
    pub kind: RelationKind,
    /// Confidence score (0.0 - 1.0) from inferrer.
    pub confidence: f64,
    /// Human-readable reasoning (for debugging/audit).
    pub reasoning: String,
    /// When this relation was inferred.
    pub inferred_at: chrono::DateTime<chrono::Utc>,
    /// When this relation expires (needs re-validation).
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// The type of logical relationship between markets.
#[derive(Debug, Clone, PartialEq)]
pub enum RelationKind {
    /// If market A resolves YES, market B must resolve YES.
    /// Relation: P(A) ≤ P(B), encoded as μ_A - μ_B ≤ 0
    Implies {
        if_yes: MarketId,
        then_yes: MarketId,
    },

    /// At most one of these markets can resolve YES.
    /// Relation: Σ μ_i ≤ 1
    MutuallyExclusive(Vec<MarketId>),

    /// Exactly one of these markets must resolve YES.
    /// Relation: Σ μ_i = 1
    ExactlyOne(Vec<MarketId>),

    /// Custom linear constraint: Σ (coeff_i × μ_i) {≤, =, ≥} rhs
    Linear {
        terms: Vec<(MarketId, Decimal)>,
        sense: ConstraintSense,
        rhs: Decimal,
    },
}

impl RelationKind {
    /// Convert to ILP constraint(s) for the solver.
    pub fn to_solver_constraints(
        &self,
        market_indices: &HashMap<MarketId, usize>,
    ) -> Vec<solver::Constraint> {
        use crate::core::solver::Constraint;
        
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
            Self::Linear { terms, sense, rhs } => {
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

    /// Get all markets referenced by this relation.
    pub fn markets(&self) -> Vec<&MarketId> {
        match self {
            Self::Implies { if_yes, then_yes } => vec![if_yes, then_yes],
            Self::MutuallyExclusive(ms) | Self::ExactlyOne(ms) => ms.iter().collect(),
            Self::Linear { terms, .. } => terms.iter().map(|(m, _)| m).collect(),
        }
    }
}

/// A cluster of related markets with pre-computed solver constraints.
#[derive(Debug, Clone)]
pub struct Cluster {
    /// Unique identifier for this cluster.
    pub id: ClusterId,
    /// Markets in this cluster (ordered for ILP variable mapping).
    pub markets: Vec<MarketId>,
    /// Source relations within this cluster.
    pub relations: Vec<Relation>,
    /// Pre-computed ILP constraints for the solver (hot path).
    pub constraints: Vec<solver::Constraint>,
    /// Last update timestamp.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Cluster {
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

/// Unique identifier for a relation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RelationId(String);

impl RelationId {
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

### Inferrer Trait

```rust
// Location: src/core/service/inference/mod.rs

use async_trait::async_trait;
use crate::core::domain::{Relation, MarketId};
use crate::core::exchange::MarketInfo;
use crate::error::Result;

/// Infers logical relations between markets.
///
/// Implementations may use LLMs, heuristic rules, or hybrid approaches.
/// All inferrers must be idempotent and safe to call repeatedly.
#[async_trait]
pub trait Inferrer: Send + Sync {
    /// Inferrer name for logging and config.
    fn name(&self) -> &'static str;

    /// Infer relations among a set of markets.
    ///
    /// # Arguments
    /// * `markets` - Market metadata to analyze
    /// * `existing` - Already-known relations (for incremental inference)
    ///
    /// # Returns
    /// New relations inferred. May overlap with `existing` (caller dedupes).
    async fn infer(
        &self,
        markets: &[MarketInfo],
        existing: &[Relation],
    ) -> Result<Vec<Relation>>;

    /// Check if this inferrer can handle the given market count.
    ///
    /// LLM inferrers may have context window limits.
    fn max_markets(&self) -> usize {
        100 // Conservative default
    }
}

/// Configuration for the inference service.
#[derive(Debug, Clone, Deserialize)]
pub struct InferenceConfig {
    /// Minimum confidence to accept a relation.
    #[serde(default = "default_min_confidence")]
    pub min_confidence: f64,

    /// How long inferred relations are valid (seconds).
    #[serde(default = "default_ttl_seconds")]
    pub ttl_seconds: u64,

    /// Price change threshold to trigger re-analysis (0.0 - 1.0).
    #[serde(default = "default_price_threshold")]
    pub price_change_threshold: f64,

    /// Maximum pending inference requests in queue.
    #[serde(default = "default_queue_size")]
    pub max_queue_size: usize,

    /// Batch size for LLM analysis.
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    /// Rate limit: max inferences per minute.
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

### Llm Trait

```rust
// Location: src/core/llm/mod.rs

/// LLM completion interface.
///
/// Implementations provide access to language models for inference tasks.
#[async_trait]
pub trait Llm: Send + Sync {
    /// Provider name for logging and config.
    fn name(&self) -> &'static str;

    /// Complete a prompt and return the response text.
    async fn complete(&self, prompt: &str) -> Result<String>;
}
```

### LLM Inferrer

```rust
// Location: src/core/service/inference/llm.rs

/// LLM-powered relation inferrer.
pub struct LlmInferrer {
    llm: Arc<dyn Llm>,
    config: LlmInferrerConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmInferrerConfig {
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
impl Inferrer for LlmInferrer {
    fn name(&self) -> &'static str {
        "llm"
    }

    async fn infer(
        &self,
        markets: &[MarketInfo],
        _existing: &[Relation],
    ) -> Result<Vec<Relation>> {
        let prompt = self.build_prompt(markets);
        let response = self.llm.complete(&prompt).await?;
        self.parse_response(&response, markets)
    }

    fn max_markets(&self) -> usize {
        // Context window considerations
        50 // ~200 tokens per market description
    }
}

impl LlmInferrer {
    fn build_prompt(&self, markets: &[MarketInfo]) -> String {
        let market_list = markets
            .iter()
            .enumerate()
            .map(|(i, m)| format!("{}. [{}] \"{}\"", i + 1, m.id, m.question))
            .collect::<Vec<_>>()
            .join("\n");

        format!(r#"Analyze these prediction markets for logical constraints.

## Markets
{market_list}

## Task
Identify logical relationships where the outcome of one market constrains another.

## Relation Types
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
  "constraints": [
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
- Only output high-confidence constraints (>0.7)
- Use market IDs exactly as provided
- Provide brief, clear reasoning
- If no constraints exist, return {{"constraints": []}}
- Do NOT invent markets or IDs
"#)
    }

    fn parse_response(
        &self,
        response: &str,
        markets: &[MarketInfo],
    ) -> Result<Vec<Relation>> {
        // Extract JSON from response (handle markdown code blocks)
        let json_str = self.extract_json(response)?;
        let parsed: LlmResponse = serde_json::from_str(&json_str)?;
        
        // Validate and convert LLM output to domain Relations
        let market_ids: HashSet<_> = markets.iter().map(|m| &m.id).collect();
        let mut relations = Vec::new();
        
        for item in parsed.constraints {
            // Validate all referenced markets exist
            if !self.validate_market_refs(&item, &market_ids) {
                tracing::warn!(item = ?item, "LLM referenced unknown market, skipping");
                continue;
            }
            
            if let Some(relation) = self.convert_to_relation(item) {
                relations.push(relation);
            }
        }
        
        Ok(relations)
    }
}
```

### Cluster Cache

```rust
// Location: src/core/cache/cluster.rs

use std::collections::HashMap;
use parking_lot::RwLock;
use chrono::{DateTime, Utc};

use crate::core::domain::{ClusterId, Relation, Cluster, MarketId};

/// Cache for inferred relations and market clusters.
///
/// Thread-safe, supports TTL-based expiration.
pub struct ClusterCache {
    /// Relations indexed by involved market.
    by_market: RwLock<HashMap<MarketId, Vec<Relation>>>,
    /// Pre-computed clusters.
    clusters: RwLock<HashMap<ClusterId, Cluster>>,
    /// Market → Cluster mapping.
    market_to_cluster: RwLock<HashMap<MarketId, ClusterId>>,
    /// Configuration.
    config: CacheConfig,
}

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub default_ttl: chrono::Duration,
    pub max_relations: usize,
    pub max_clusters: usize,
}

impl ClusterCache {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            by_market: RwLock::new(HashMap::new()),
            clusters: RwLock::new(HashMap::new()),
            market_to_cluster: RwLock::new(HashMap::new()),
            config,
        }
    }

    /// Get cluster for a market (if any relations exist).
    pub fn get_cluster(&self, market_id: &MarketId) -> Option<Cluster> {
        let mapping = self.market_to_cluster.read();
        let cluster_id = mapping.get(market_id)?;
        
        let clusters = self.clusters.read();
        let cluster = clusters.get(cluster_id)?;
        
        // Check expiration
        if cluster.updated_at + self.config.default_ttl < Utc::now() {
            return None; // Expired
        }
        
        Some(cluster.clone())
    }

    /// Add new relations and rebuild affected clusters.
    pub fn add_relations(&self, relations: Vec<Relation>) {
        if relations.is_empty() {
            return;
        }

        // Group by cluster (using union-find for connected components)
        let clusters = self.build_clusters(&relations);
        
        // Update cache atomically
        let mut by_market = self.by_market.write();
        let mut cluster_cache = self.clusters.write();
        let mut mapping = self.market_to_cluster.write();
        
        for rel in &relations {
            for market_id in rel.kind.markets() {
                by_market
                    .entry(market_id.clone())
                    .or_default()
                    .push(rel.clone());
            }
        }
        
        for cluster in clusters {
            for market_id in &cluster.markets {
                mapping.insert(market_id.clone(), cluster.id.clone());
            }
            cluster_cache.insert(cluster.id.clone(), cluster);
        }
    }

    /// Invalidate relations involving a market.
    pub fn invalidate(&self, market_id: &MarketId) {
        let mut by_market = self.by_market.write();
        by_market.remove(market_id);
        
        // Also invalidate cluster
        let mut mapping = self.market_to_cluster.write();
        if let Some(cluster_id) = mapping.remove(market_id) {
            let mut clusters = self.clusters.write();
            clusters.remove(&cluster_id);
        }
    }

    /// Build clusters from relations using union-find.
    fn build_clusters(&self, relations: &[Relation]) -> Vec<Cluster> {
        // Union-find to group connected markets
        let mut uf = UnionFind::new();
        
        for rel in relations {
            let markets: Vec<_> = rel.kind.markets().into_iter().cloned().collect();
            for window in markets.windows(2) {
                uf.union(&window[0], &window[1]);
            }
        }
        
        // Group relations by cluster root
        let mut cluster_rels: HashMap<MarketId, Vec<Relation>> = HashMap::new();
        for rel in relations {
            let root = uf.find(rel.kind.markets()[0]);
            cluster_rels.entry(root.clone()).or_default().push(rel.clone());
        }
        
        // Build Cluster for each group
        cluster_rels
            .into_iter()
            .map(|(_, rels)| self.build_single_cluster(rels))
            .collect()
    }

    fn build_single_cluster(&self, relations: Vec<Relation>) -> Cluster {
        // Collect all unique markets
        let mut markets: Vec<MarketId> = relations
            .iter()
            .flat_map(|r| r.kind.markets().into_iter().cloned())
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
        
        // Convert relations to solver constraints
        let constraints: Vec<_> = relations
            .iter()
            .flat_map(|r| r.kind.to_solver_constraints(&market_indices))
            .collect();
        
        Cluster {
            id: ClusterId::new(),
            markets,
            relations,
            constraints,
            updated_at: Utc::now(),
        }
    }
}
```

### Inference Service

```rust
// Location: src/core/service/inference/service.rs

use std::sync::Arc;
use tokio::sync::mpsc;

/// Event that triggers relation inference.
#[derive(Debug, Clone)]
pub enum InferenceEvent {
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

/// The main relation inference service.
pub struct InferenceService {
    /// The inferrer implementation.
    inferrer: Arc<dyn Inferrer>,
    /// Cache for results.
    cache: Arc<ClusterCache>,
    /// Configuration.
    config: InferenceConfig,
    /// Event receiver.
    event_rx: mpsc::Receiver<InferenceEvent>,
    /// Event sender (cloned to event sources).
    event_tx: mpsc::Sender<InferenceEvent>,
    /// All known markets for batch analysis.
    known_markets: RwLock<Vec<MarketInfo>>,
}

impl InferenceService {
    pub fn new(
        inferrer: Arc<dyn Inferrer>,
        cache: Arc<ClusterCache>,
        config: InferenceConfig,
    ) -> Self {
        let (event_tx, event_rx) = mpsc::channel(config.max_queue_size);
        Self {
            inferrer,
            cache,
            config,
            event_rx,
            event_tx,
            known_markets: RwLock::new(Vec::new()),
        }
    }

    /// Get a sender for inference events.
    pub fn event_sender(&self) -> mpsc::Sender<InferenceEvent> {
        self.event_tx.clone()
    }

    /// Run the inference service (call from orchestrator).
    pub async fn run(&mut self) {
        let mut rate_limiter = RateLimiter::new(self.config.rate_limit_per_minute);
        let mut batch: Vec<MarketInfo> = Vec::new();
        let mut batch_timeout = tokio::time::interval(Duration::from_secs(5));

        loop {
            tokio::select! {
                Some(event) = self.event_rx.recv() => {
                    match event {
                        InferenceEvent::NewMarket(info) => {
                            self.known_markets.write().push(info.clone());
                            batch.push(info);
                        }
                        InferenceEvent::PriceChange { market_id, .. } => {
                            // Invalidate and re-queue for inference
                            self.cache.invalidate(&MarketId::from(market_id.as_str()));
                            if let Some(info) = self.get_market_info(&market_id) {
                                batch.push(info);
                            }
                        }
                        InferenceEvent::FullScan => {
                            batch = self.known_markets.read().clone();
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
        let existing = self.get_existing_relations(&markets);

        match self.inferrer.infer(&markets, &existing).await {
            Ok(relations) => {
                // Filter by confidence
                let valid: Vec<_> = relations
                    .into_iter()
                    .filter(|r| r.confidence >= self.config.min_confidence)
                    .collect();

                if !valid.is_empty() {
                    tracing::info!(
                        count = valid.len(),
                        "Inferred new relations"
                    );
                    self.cache.add_relations(valid);
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "Inference failed, will retry");
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
        self.config.enabled && ctx.has_relations
    }

    fn detect(&self, ctx: &DetectionContext) -> Vec<Opportunity> {
        // Get cluster from cache
        let cluster = match self.cluster_cache.get_cluster(ctx.market.market_id()) {
            Some(c) => c,
            None => return vec![], // No known constraints
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
    /// Create context with cluster cache lookup.
    pub fn new_with_constraints(
        market: &'a Market,
        cache: &'a OrderBookCache,
        cluster_cache: &'a ClusterCache,
    ) -> Self {
        let has_deps = cluster_cache.get_cluster(market.market_id()).is_some();
        let correlated = if has_deps {
            cluster_cache
                .get_cluster(market.market_id())
                .map(|c| c.markets.clone())
                .unwrap_or_default()
        } else {
            vec![]
        };

        let market_ctx = MarketContext {
            outcome_count: market.outcome_count(),
            has_relations: has_deps,
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
    inference_tx: mpsc::Sender<InferenceEvent>,
}

impl PriceTracker {
    fn check_and_update(&mut self, token_id: &TokenId, new_price: Decimal) {
        if let Some(old_price) = self.last_prices.get(token_id) {
            let change = ((new_price - old_price) / old_price).abs();
            if change > self.threshold {
                // Trigger inference
                let _ = self.inference_tx.try_send(InferenceEvent::PriceChange {
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

        // Initialize relation inference
        let cluster_cache = Arc::new(ClusterCache::new(CacheConfig {
            default_ttl: chrono::Duration::seconds(config.inference.ttl_seconds as i64),
            max_constraints: 10_000,
            max_clusters: 1_000,
        }));

        let inferrer: Arc<dyn Inferrer> = match &config.inference.provider {
            "llm" => Arc::new(LlmInferrer::new(config.inference.llm.clone())),
            "rules" => Arc::new(RuleInferrer::new()),
            _ => Arc::new(NullInferrer),
        };

        let mut inference_service = InferenceService::new(
            inferrer,
            cluster_cache.clone(),
            config.inference.clone(),
        );

        // Spawn inference service
        let inference_tx = inference_service.event_sender();
        tokio::spawn(async move {
            inference_service.run().await;
        });

        // Queue initial full scan
        inference_tx.send(InferenceEvent::FullScan).await?;

        // ... rest of initialization ...
    }
}
```

---

## Configuration

### Config File Additions

```toml
# config.toml

[inference]
# Inferrer backend: "llm", "rules", "hybrid", "null"
provider = "llm"

# Minimum confidence to accept (0.0 - 1.0)
min_confidence = 0.7

# Relation TTL in seconds (3600 = 1 hour)
ttl_seconds = 3600

# Price change threshold to trigger re-analysis (5%)
price_change_threshold = 0.05

# Event queue size
max_queue_size = 1000

# Batch size for LLM calls
batch_size = 20

# Rate limit: max inferences per minute
rate_limit_per_minute = 30

[inference.llm]
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
| Unknown market ID | Validation | Skip that relation |
| Low confidence | Threshold check | Filter out |
| Timeout | Request timeout | Retry with backoff |
| Rate limit | HTTP 429 | Exponential backoff |
| Hallucinated relationship | Semantic validation | Cannot detect - rely on confidence |

### Safety Guarantees

1. **No direct trade decisions from LLM** — LLM only suggests constraints, solver validates
2. **Confidence filtering** — Only high-confidence (>0.7) constraints used
3. **Cache TTL** — Stale constraints expire, forcing re-validation
4. **Solver verification** — Frank-Wolfe won't produce invalid trades from bad constraints

---

## Module Structure

```
src/
├── core/
│   ├── db/                     # NEW: Database layer (Diesel)
│   │   ├── mod.rs              # Connection pool, re-exports
│   │   ├── schema.rs           # diesel::table! macros (auto-generated)
│   │   └── model.rs            # Insertable/Queryable structs
│   │
│   ├── store/                  # NEW: Persistence abstraction
│   │   ├── mod.rs              # Store<T> trait
│   │   ├── sqlite.rs           # SqliteStore (uses core::db)
│   │   └── memory.rs           # MemoryStore (for tests)
│   │
│   ├── domain/
│   │   ├── mod.rs              # Add relation exports
│   │   └── relation.rs         # NEW: Relation, RelationKind, Cluster
│   │
│   ├── cache/
│   │   ├── mod.rs              # Add ClusterCache export
│   │   └── cluster.rs          # NEW: ClusterCache (uses Store)
│   │
│   ├── llm/                    # NEW: LLM abstraction
│   │   ├── mod.rs              # Llm trait
│   │   ├── anthropic.rs        # AnthropicLlm
│   │   └── openai.rs           # OpenAiLlm
│   │
│   ├── service/
│   │   ├── mod.rs              # Add inference exports
│   │   └── inference/          # NEW: Inference service
│   │       ├── mod.rs          # Inferrer trait + config
│   │       ├── llm.rs          # LlmInferrer
│   │       ├── rules.rs        # RuleInferrer (heuristics)
│   │       └── service.rs      # InferenceService coordinator
│   │
│   └── strategy/
│       └── combinatorial/
│           └── mod.rs          # Update detect() implementation
│
├── app/
│   ├── config/
│   │   ├── mod.rs              # Add LlmConfig, InferenceConfig exports
│   │   ├── llm.rs              # NEW: LlmConfig, AnthropicConfig, OpenAiConfig
│   │   └── inference.rs        # NEW: InferenceConfig
│   │
│   └── orchestrator/
│       ├── mod.rs              # Wire up inference service
│       └── handler.rs          # Add price change tracking
│
└── migrations/                 # Diesel migrations (repo root)
    ├── 00000000000000_diesel_setup/
    └── 2026020401_create_relations/
```

---

## Implementation Plan

### Prerequisites: Dependencies & Setup

**Cargo.toml additions:**
```toml
# Database
diesel = { version = "2", features = ["sqlite", "r2d2", "chrono"] }
diesel_migrations = "2"

# UUID for IDs
uuid = { version = "1", features = ["v4", "serde"] }

# Union-find for clustering
petgraph = "0.6"  # or implement simple union-find
```

**Environment variables:**
```bash
# Required for LLM inference
ANTHROPIC_API_KEY=sk-ant-...
# OR
OPENAI_API_KEY=sk-...

# Database path (optional, defaults to ./data/edgelord.db)
DATABASE_URL=sqlite://./data/edgelord.db
```

**New error variants in `src/error.rs`:**
```rust
#[derive(Error, Debug)]
pub enum LlmError {
    #[error("LLM request failed: {0}")]
    RequestFailed(String),
    #[error("LLM response parse error: {0}")]
    ParseError(String),
    #[error("LLM rate limited, retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },
    #[error("invalid API key")]
    InvalidApiKey,
}

#[derive(Error, Debug)]
pub enum InferenceError {
    #[error("inference failed: {0}")]
    Failed(String),
    #[error("no inferrer configured")]
    NoInferrer,
}

#[derive(Error, Debug)]
pub enum StoreError {
    #[error("database error: {0}")]
    Database(#[from] diesel::result::Error),
    #[error("connection pool error: {0}")]
    Pool(String),
}

// Add to main Error enum:
#[error(transparent)]
Llm(#[from] LlmError),
#[error(transparent)]
Inference(#[from] InferenceError),
#[error(transparent)]
Store(#[from] StoreError),
```

---

### Phase 1: Domain Types (Day 1, Morning)

**Files to create:**
- `src/core/domain/relation.rs`

**Files to modify:**
- `src/core/domain/mod.rs` — add `pub mod relation; pub use relation::*;`
- `src/core/domain/id.rs` — add `RelationId`, `ClusterId`

**`src/core/domain/id.rs` additions:**
```rust
/// Unique identifier for an inferred relation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RelationId(String);

impl RelationId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for RelationId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RelationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for RelationId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

// Same pattern for ClusterId
```

**Tests:**
- `RelationKind::to_solver_constraints()` produces correct coefficients
- `Cluster` builds valid ILP problems
- Serialization round-trips

---

### Phase 2: Database Layer (Day 1, Afternoon)

**Files to create:**
- `src/core/db/mod.rs`
- `src/core/db/schema.rs` (auto-generated by diesel)
- `src/core/db/model.rs`
- `migrations/2026020401_create_relations/up.sql`
- `migrations/2026020401_create_relations/down.sql`

**Files to modify:**
- `src/core/mod.rs` — add `pub mod db;`

**Migration SQL (`up.sql`):**
```sql
CREATE TABLE relations (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL,  -- JSON: {"Implies": {"if_yes": "...", "then_yes": "..."}}
    confidence REAL NOT NULL,
    reasoning TEXT NOT NULL,
    inferred_at TEXT NOT NULL,  -- ISO 8601
    expires_at TEXT NOT NULL,
    
    -- Denormalized for queries
    market_ids TEXT NOT NULL  -- JSON array: ["market_1", "market_2"]
);

CREATE INDEX idx_relations_expires_at ON relations(expires_at);
CREATE INDEX idx_relations_market_ids ON relations(market_ids);

CREATE TABLE clusters (
    id TEXT PRIMARY KEY NOT NULL,
    market_ids TEXT NOT NULL,  -- JSON array
    relation_ids TEXT NOT NULL,  -- JSON array
    constraints_json TEXT NOT NULL,  -- Pre-computed solver constraints
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_clusters_updated_at ON clusters(updated_at);
```

**`src/core/db/model.rs`:**
```rust
use diesel::prelude::*;

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::core::db::schema::relations)]
pub struct RelationRow {
    pub id: String,
    pub kind: String,  // JSON
    pub confidence: f64,
    pub reasoning: String,
    pub inferred_at: String,
    pub expires_at: String,
    pub market_ids: String,  // JSON
}

// Conversion: RelationRow <-> domain::Relation
```

**`src/core/db/mod.rs`:**
```rust
pub mod model;
pub mod schema;

use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;

pub fn create_pool(database_url: &str) -> Result<DbPool, StoreError> {
    let manager = ConnectionManager::<SqliteConnection>::new(database_url);
    Pool::builder()
        .max_size(5)
        .build(manager)
        .map_err(|e| StoreError::Pool(e.to_string()))
}
```

**Tests:**
- Migration runs cleanly
- Insert/query/delete relations
- Insert/query clusters

---

### Phase 3: Store Layer (Day 2, Morning)

**Files to create:**
- `src/core/store/mod.rs`
- `src/core/store/sqlite.rs`
- `src/core/store/memory.rs`

**Files to modify:**
- `src/core/mod.rs` — add `pub mod store;`

**`src/core/store/mod.rs`:**
```rust
mod memory;
mod sqlite;

pub use memory::MemoryStore;
pub use sqlite::SqliteStore;

use async_trait::async_trait;
use crate::error::Result;

/// Generic key-value store for persistence.
#[async_trait]
pub trait Store<T>: Send + Sync {
    /// Get item by key.
    async fn get(&self, key: &str) -> Result<Option<T>>;
    
    /// Put item (upsert).
    async fn put(&self, key: &str, value: &T) -> Result<()>;
    
    /// Delete item.
    async fn delete(&self, key: &str) -> Result<()>;
    
    /// List all keys.
    async fn keys(&self) -> Result<Vec<String>>;
    
    /// List items matching a predicate.
    async fn list<F>(&self, predicate: F) -> Result<Vec<T>>
    where
        F: Fn(&T) -> bool + Send + Sync;
}
```

**`src/core/store/sqlite.rs`:**
```rust
pub struct SqliteRelationStore {
    pool: DbPool,
}

impl SqliteRelationStore {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
    
    /// Get relations involving a specific market.
    pub fn get_by_market(&self, market_id: &MarketId) -> Result<Vec<Relation>> {
        // Query with LIKE on market_ids JSON
    }
    
    /// Delete expired relations.
    pub fn prune_expired(&self) -> Result<usize> {
        // DELETE WHERE expires_at < now
    }
}

#[async_trait]
impl Store<Relation> for SqliteRelationStore { ... }
```

**Tests:**
- Store CRUD operations
- `get_by_market` returns correct relations
- `prune_expired` removes old entries

---

### Phase 4: Cache Layer (Day 2, Afternoon)

**Files to create:**
- `src/core/cache/cluster.rs`

**Files to modify:**
- `src/core/cache/mod.rs` — add `pub mod cluster; pub use cluster::ClusterCache;`

**Key implementation details:**
- Union-find for building clusters from relations
- In-memory HashMap for hot path, backed by Store for persistence
- TTL-based expiration
- Thread-safe with `parking_lot::RwLock`

**Tests:**
- Cluster building from relations
- TTL expiration
- Concurrent access

---

### Phase 5: LLM Module (Day 3)

**Files to create:**
- `src/core/llm/mod.rs`
- `src/core/llm/anthropic.rs`
- `src/core/llm/openai.rs`

**Files to modify:**
- `src/core/mod.rs` — add `pub mod llm;`

**`src/core/llm/mod.rs`:**
```rust
mod anthropic;
mod openai;

pub use anthropic::AnthropicLlm;
pub use openai::OpenAiLlm;

use async_trait::async_trait;
use crate::error::Result;

#[async_trait]
pub trait Llm: Send + Sync {
    fn name(&self) -> &'static str;
    async fn complete(&self, prompt: &str) -> Result<String>;
}
```

**`src/core/llm/anthropic.rs`:**
```rust
pub struct AnthropicLlm {
    client: reqwest::Client,
    api_key: String,
    model: String,
    max_tokens: usize,
    temperature: f64,
}

impl AnthropicLlm {
    pub fn new(config: &AnthropicConfig) -> Result<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| LlmError::InvalidApiKey)?;
        // ...
    }
}

#[async_trait]
impl Llm for AnthropicLlm {
    fn name(&self) -> &'static str { "anthropic" }
    
    async fn complete(&self, prompt: &str) -> Result<String> {
        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&serde_json::json!({
                "model": self.model,
                "max_tokens": self.max_tokens,
                "temperature": self.temperature,
                "messages": [{"role": "user", "content": prompt}]
            }))
            .send()
            .await?;
        // Parse response, handle errors
    }
}
```

**Tests:**
- Mock HTTP responses
- Error handling (rate limits, invalid key, parse errors)

---

### Phase 6: Config Layer (Day 3-4)

**Files to create:**
- `src/app/config/llm.rs`
- `src/app/config/inference.rs`

**Files to modify:**
- `src/app/config/mod.rs` — add modules and exports, add to `Config` struct

**`src/app/config/llm.rs`:**
```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "provider", rename_all = "lowercase")]
pub enum LlmConfig {
    Anthropic(AnthropicConfig),
    OpenAi(OpenAiConfig),
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicConfig {
    #[serde(default = "default_anthropic_model")]
    pub model: String,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
}

fn default_anthropic_model() -> String { "claude-3-5-sonnet-20241022".to_string() }
fn default_temperature() -> f64 { 0.3 }
fn default_max_tokens() -> usize { 4096 }
```

**`src/app/config/inference.rs`:**
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct InferenceConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_provider")]
    pub provider: InferenceProvider,
    #[serde(default = "default_min_confidence")]
    pub min_confidence: f64,
    #[serde(default = "default_ttl_seconds")]
    pub ttl_seconds: u64,
    #[serde(default = "default_price_threshold")]
    pub price_change_threshold: f64,
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_minute: u32,
    #[serde(default)]
    pub llm: Option<LlmConfig>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InferenceProvider {
    #[default]
    Null,
    Llm,
    Rules,
}
```

**Add to `Config` struct:**
```rust
pub struct Config {
    // ... existing fields ...
    #[serde(default)]
    pub inference: InferenceConfig,
    #[serde(default)]
    pub database_url: Option<String>,
}
```

---

### Phase 7: Inference Service (Day 4)

**Files to create:**
- `src/core/service/inference/mod.rs`
- `src/core/service/inference/llm.rs`
- `src/core/service/inference/rules.rs`
- `src/core/service/inference/service.rs`

**Files to modify:**
- `src/core/service/mod.rs` — add `pub mod inference; pub use inference::*;`

**Key types:**
- `Inferrer` trait
- `LlmInferrer` — uses `Llm` trait, builds prompts, parses responses
- `RuleInferrer` — heuristic patterns (same question text = mutually exclusive)
- `NullInferrer` — no-op for disabled inference
- `InferenceService` — event-driven coordinator
- `InferenceEvent` — NewMarket, PriceChange, FullScan

**Tests:**
- LlmInferrer prompt construction
- LlmInferrer response parsing (valid JSON, invalid JSON, edge cases)
- RuleInferrer pattern matching
- InferenceService batching logic

---

### Phase 8: Orchestrator Integration (Day 5)

**Files to modify:**
- `src/app/orchestrator/mod.rs`
- `src/app/orchestrator/builder.rs`
- `src/app/orchestrator/handler.rs`

**`src/app/orchestrator/builder.rs` additions:**
```rust
/// Build LLM client from configuration.
pub(crate) fn build_llm(config: &Config) -> Option<Arc<dyn Llm>> {
    config.inference.llm.as_ref().map(|llm_config| {
        match llm_config {
            LlmConfig::Anthropic(c) => Arc::new(AnthropicLlm::new(c).unwrap()) as Arc<dyn Llm>,
            LlmConfig::OpenAi(c) => Arc::new(OpenAiLlm::new(c).unwrap()) as Arc<dyn Llm>,
        }
    })
}

/// Build inferrer from configuration.
pub(crate) fn build_inferrer(config: &Config, llm: Option<Arc<dyn Llm>>) -> Arc<dyn Inferrer> {
    match config.inference.provider {
        InferenceProvider::Llm => {
            let llm = llm.expect("LLM config required for LLM inferrer");
            Arc::new(LlmInferrer::new(llm, config.inference.clone()))
        }
        InferenceProvider::Rules => Arc::new(RuleInferrer::new()),
        InferenceProvider::Null => Arc::new(NullInferrer),
    }
}

/// Build cluster cache with optional persistence.
pub(crate) fn build_cluster_cache(config: &Config) -> Arc<ClusterCache> {
    let store = config.database_url.as_ref().map(|url| {
        let pool = crate::core::db::create_pool(url).expect("Failed to create DB pool");
        Arc::new(SqliteRelationStore::new(pool)) as Arc<dyn Store<Relation>>
    });
    Arc::new(ClusterCache::new(CacheConfig {
        default_ttl: chrono::Duration::seconds(config.inference.ttl_seconds as i64),
        max_relations: 10_000,
        max_clusters: 1_000,
    }, store))
}
```

**`src/app/orchestrator/mod.rs` additions:**
```rust
pub async fn run(config: Config) -> Result<()> {
    // ... existing initialization ...
    
    // Initialize inference system (if enabled)
    let cluster_cache = build_cluster_cache(&config);
    let inference_tx = if config.inference.enabled {
        let llm = build_llm(&config);
        let inferrer = build_inferrer(&config, llm);
        let mut inference_service = InferenceService::new(
            inferrer,
            cluster_cache.clone(),
            config.inference.clone(),
        );
        let tx = inference_service.event_sender();
        
        // Spawn inference service task
        tokio::spawn(async move {
            inference_service.run().await;
        });
        
        // Queue initial full scan with all markets
        tx.send(InferenceEvent::FullScan(market_infos.clone())).await.ok();
        
        Some(tx)
    } else {
        None
    };
    
    // Pass cluster_cache to strategies that need it
    let strategies = Arc::new(build_strategy_registry(&config, cluster_cache.clone()));
    
    // ... rest of initialization ...
}
```

**`src/app/orchestrator/handler.rs` additions:**
```rust
/// Track prices for change detection.
struct PriceTracker {
    last_prices: HashMap<TokenId, Decimal>,
    threshold: Decimal,
}

impl PriceTracker {
    fn check(&mut self, token_id: &TokenId, new_price: Decimal) -> Option<PriceChange> {
        if let Some(old_price) = self.last_prices.get(token_id) {
            let change = ((new_price - *old_price) / *old_price).abs();
            if change > self.threshold {
                let result = PriceChange {
                    token_id: token_id.clone(),
                    old_price: *old_price,
                    new_price,
                };
                self.last_prices.insert(token_id.clone(), new_price);
                return Some(result);
            }
        }
        self.last_prices.insert(token_id.clone(), new_price);
        None
    }
}
```

---

### Phase 9: Strategy Update (Day 5)

**Files to modify:**
- `src/core/strategy/combinatorial/mod.rs`
- `src/core/strategy/context.rs`
- `src/app/orchestrator/builder.rs`

**`CombinatorialStrategy` changes:**
```rust
pub struct CombinatorialStrategy {
    config: CombinatorialConfig,
    cluster_cache: Arc<ClusterCache>,  // NEW
    fw: FrankWolfe,
    solver: Solver,
}

impl CombinatorialStrategy {
    pub fn new(config: CombinatorialConfig, cluster_cache: Arc<ClusterCache>) -> Self {
        Self {
            config,
            cluster_cache,
            fw: FrankWolfe::new(config.frank_wolfe.clone()),
            solver: Solver::new(),
        }
    }
}

impl Strategy for CombinatorialStrategy {
    fn applies_to(&self, ctx: &MarketContext) -> bool {
        self.config.enabled && 
        self.cluster_cache.get_cluster(ctx.market_id).is_some()
    }
    
    fn detect(&self, ctx: &DetectionContext) -> Vec<Opportunity> {
        let cluster = match self.cluster_cache.get_cluster(ctx.market.market_id()) {
            Some(c) => c,
            None => return vec![],
        };
        
        // ... existing Frank-Wolfe logic using cluster.constraints ...
    }
}
```

**`build_strategy_registry` changes:**
```rust
pub(crate) fn build_strategy_registry(
    config: &Config, 
    cluster_cache: Arc<ClusterCache>
) -> StrategyRegistry {
    // ...
    "combinatorial" => {
        if config.strategies.combinatorial.enabled {
            registry.register(Box::new(CombinatorialStrategy::new(
                config.strategies.combinatorial.clone(),
                cluster_cache.clone(),
            )));
        }
    }
    // ...
}
```

---

### Phase 10: End-to-End Testing (Day 6)

**Integration tests to write:**
1. **Full flow test**: Mock market data → inference → cluster → strategy detection
2. **Persistence test**: Relations survive restart
3. **Expiration test**: TTL correctly expires relations
4. **Price change test**: Significant price changes trigger re-inference
5. **Error handling**: LLM failures don't crash the system

**Test files:**
- `tests/inference_integration.rs`
- `tests/cluster_persistence.rs`

---

### Phase 11: Documentation & Polish (Day 6)

**Files to create/update:**
- Update `README.md` with inference configuration
- Update `config.toml.example` with inference section
- Add rustdoc to all public types
- Update `ARCHITECTURE.md` if needed

**Metrics/logging:**
- Add `tracing` spans for inference operations
- Log inference latency, success rate
- Log cluster cache hit rate

---

## Resolved Decisions

| Question | Decision | Rationale |
|----------|----------|-----------|
| LLM Provider | Claude 3.5 Sonnet (default) | Best JSON output quality, consistent |
| Confidence Threshold | 0.7 (configurable) | Start conservative, tune with data |
| Full Scan Frequency | Hourly + on >10 new markets | Balance freshness vs cost |
| Multi-Exchange | Out of scope v1 | Architecture supports it later |
| Persistence | SQLite via Diesel | Survive restarts, query by market |
| Cache TTL | 1 hour (configurable) | Force periodic re-validation |

---

## References

- [Arbitrage-Free Combinatorial Market Making (arXiv:1606.02825)](https://arxiv.org/abs/1606.02825)
- [Unravelling the Probabilistic Forest (arXiv:2508.03474)](https://arxiv.org/abs/2508.03474)
- Existing: `doc/research/combinatorial-future.md`
- Existing: `src/core/strategy/combinatorial/`
