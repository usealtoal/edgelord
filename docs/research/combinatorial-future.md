# Combinatorial Strategy: Future Work

The combinatorial strategy infrastructure is implemented but not active. This document captures ideas for completing it.

## What's Done

- **Bregman divergence** — KL divergence for LMSR markets (`bregman.rs`)
- **Frank-Wolfe algorithm** — Iterative projection onto marginal polytope (`frank_wolfe.rs`)
- **HiGHS solver** — LP/ILP solving via `good_lp` crate (`solver/`)
- **Strategy trait** — Warm-start support, detection context

The math works. Tests pass. The gap is knowing which markets are related.

## What's Missing

### Dependency Representation

Current `MarketContext` has placeholder fields:
```rust
has_dependencies: bool,
correlated_markets: Vec<MarketId>,
```

Need structured constraints:
```rust
enum DependencyConstraint {
    // P(A) implies P(B), so P(A) ≤ P(B)
    Implies { if_market: MarketId, then_market: MarketId },

    // Outcomes are mutually exclusive
    MutuallyExclusive(Vec<MarketId>),

    // Custom linear constraint
    Linear { coefficients: Vec<f64>, sense: Sense, rhs: f64 },
}
```

### ILP Constraint Builder

Convert dependency graph → LP constraints:
- `Implies(A, B)` → μ_A - μ_B ≤ 0
- `MutuallyExclusive([A, B, C])` → μ_A + μ_B + μ_C ≤ 1

### Multi-Market Price Aggregation

`DetectionContext` currently sees one market. Need to gather prices from all correlated markets to build the joint price vector θ for Frank-Wolfe.

## LLM-Assisted Dependency Discovery

The research paper used LLMs to identify market relationships. Approach:

### 1. Fetch Market Questions

From Polymarket API, get active market questions:
```
- "Will Trump win Pennsylvania?"
- "Will Republicans win PA by 5+ points?"
- "Will Democrats win PA?"
- "Who wins the 2024 election?"
```

### 2. Query LLM for Dependencies

Prompt structure:
```
Given these prediction market questions, identify logical dependencies.

Markets:
1. [id: 0x123] "Will Trump win Pennsylvania?"
2. [id: 0x456] "Will Republicans win PA by 5+ points?"
3. [id: 0x789] "Will Democrats win PA?"

For each dependency, specify:
- Type: implies, mutually_exclusive, or custom
- Markets involved
- Reasoning

Respond in JSON.
```

### 3. Parse and Cache

```rust
struct DiscoveredDependency {
    constraint: DependencyConstraint,
    confidence: f64,
    reasoning: String,
    discovered_at: DateTime<Utc>,
}
```

Cache results. Re-scan periodically as new markets appear.

### 4. Human Review (Optional)

High-value dependencies could be flagged for review before use. LLM might hallucinate relationships.

## Implementation Sketch

```rust
// New module: src/application/strategy/combinatorial/discovery.rs

pub struct DependencyDiscovery {
    llm_client: LlmClient,
    cache: DependencyCache,
}

impl DependencyDiscovery {
    /// Scan markets and discover dependencies
    pub async fn discover(&self, markets: &[MarketInfo]) -> Vec<DiscoveredDependency> {
        let questions: Vec<_> = markets.iter()
            .map(|m| (m.id.clone(), m.question.clone()))
            .collect();

        let prompt = self.build_prompt(&questions);
        let response = self.llm_client.complete(&prompt).await?;
        self.parse_dependencies(&response)
    }

    /// Build market clusters from dependencies
    pub fn build_clusters(&self, deps: &[DiscoveredDependency]) -> Vec<MarketCluster> {
        // Union-find or graph traversal to group related markets
    }
}

// In CombinatorialStrategy::detect()
fn detect(&self, ctx: &DetectionContext) -> Vec<Opportunity> {
    let cluster = ctx.market_cluster()?;
    let prices = cluster.aggregate_prices();
    let constraints = cluster.build_ilp_constraints();

    let result = frank_wolfe(&prices, &constraints, &self.config);

    if result.gap > self.config.gap_threshold {
        vec![self.create_opportunity(&result, &cluster)]
    } else {
        vec![]
    }
}
```

## Open Questions

1. **Which LLM?** Claude API, local model, or configurable?
2. **Rate of discovery?** On startup? Periodic refresh? On new market event?
3. **Confidence threshold?** When to trust LLM-discovered dependencies?
4. **Execution complexity?** Multi-market opportunities need coordinated orders

## Why Bother?

Historical share: 0.24% ($95K of $40M). Small, but:
- Opportunities may be higher-value per trade
- Less competition (sophisticated strategy)
- Interesting research direction
- Infrastructure is already built

## References

- [Arbitrage-Free Combinatorial Market Making via Integer Programming](https://arxiv.org/abs/1606.02825)
- [Unravelling the Probabilistic Forest](https://arxiv.org/abs/2508.03474) — LLM-assisted approach
