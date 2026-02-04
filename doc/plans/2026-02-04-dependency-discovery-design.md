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

### Phase 1: Domain Types + DB Layer (Day 1)
- [ ] Add `Relation`, `RelationKind`, `Cluster` to domain
- [ ] Add `RelationId`, `ClusterId` to id.rs
- [ ] Set up Diesel: `diesel setup`, initial migration
- [ ] Create `core/db/` with schema.rs, models.rs
- [ ] Write unit tests for constraint types

### Phase 2: Store Layer (Day 1-2)
- [ ] Define `Store<T>` trait in `core/store/mod.rs`
- [ ] Implement `SqliteStore` using `core::db`
- [ ] Implement `MemoryStore` for tests
- [ ] Write tests for store operations

### Phase 3: Cache Layer (Day 2)
- [ ] Implement `ClusterCache` with TTL support (uses Store)
- [ ] Implement union-find for cluster building
- [ ] Write tests for cache operations

### Phase 4: LLM Module (Day 2-3)
- [ ] Define `Llm` trait in `core/llm/mod.rs`
- [ ] Implement `AnthropicLlm`
- [ ] Implement `OpenAiLlm`
- [ ] Add config in `app/config/llm.rs`

### Phase 5: Inference Service (Day 3-4)
- [ ] Define `Inferrer` trait
- [ ] Implement `LlmInferrer` with prompt engineering
- [ ] Implement `RuleInferrer` (heuristics)
- [ ] Add response parsing and validation
- [ ] Write integration tests with mock LLM

### Phase 6: Service Integration (Day 4-5)
- [ ] Implement `InferenceService` coordinator
- [ ] Add `app/config/inference.rs`
- [ ] Wire into orchestrator
- [ ] Add price change tracking

### Phase 7: Strategy Update (Day 5)
- [ ] Update `CombinatorialStrategy::detect()`
- [ ] Update `DetectionContext` with constraint lookup
- [ ] End-to-end testing

### Phase 8: Polish (Day 6)
- [ ] Documentation
- [ ] Metrics/logging
- [ ] Configuration validation
- [ ] Edge case handling

---

## Open Questions

1. **LLM Provider Priority:** Claude vs GPT-4 vs local model?
   - Claude 3.5 Sonnet recommended for consistency and JSON output quality

2. **Confidence Calibration:** How to tune 0.7 threshold?
   - Start conservative, lower if missing real constraints

3. **Full Scan Frequency:** How often to re-analyze all markets?
   - Suggest: hourly, or on significant market events (>10 new markets)

4. **Multi-Exchange Relations:** Cross-exchange correlations?
   - Out of scope for v1, but architecture supports it

5. **Persistence:** Should constraints survive restarts?
   - Suggest: Yes, add simple file-based persistence

---

## References

- [Arbitrage-Free Combinatorial Market Making (arXiv:1606.02825)](https://arxiv.org/abs/1606.02825)
- [Unravelling the Probabilistic Forest (arXiv:2508.03474)](https://arxiv.org/abs/2508.03474)
- Existing: `doc/research/combinatorial-future.md`
- Existing: `src/core/strategy/combinatorial/`
