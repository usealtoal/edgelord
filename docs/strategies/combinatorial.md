# Combinatorial Strategy

Detects arbitrage across correlated markets using LLM-powered relation inference and Frank-Wolfe optimization.

## What It Detects

Some markets have logical dependencies. If Market A implies Market B, their prices must satisfy certain constraints. Violations of these constraints create arbitrage.

**Example:**
- Market A: "Trump wins Pennsylvania" — YES at $0.45
- Market B: "Trump wins any swing state" — YES at $0.40

If Trump wins PA, he must win a swing state. So P(A) ≤ P(B). But $0.45 > $0.40 — **violation!** This can be profitably traded.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      STARTUP                                    │
├─────────────────────────────────────────────────────────────────┤
│  Markets ──► LlmInferrer ──► Relations ──► ClusterCache         │
│              (Anthropic/     (implies,      (pre-computed       │
│               OpenAI)        mutex, etc.)    constraints)       │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                      RUNTIME                                    │
├─────────────────────────────────────────────────────────────────┤
│  OrderBook Updates ──► ClusterDetectionService                  │
│                              │                                  │
│                              ├── tracks dirty clusters          │
│                              ├── debounces (100ms)              │
│                              └── ClusterDetector                │
│                                       │                         │
│                                       ▼                         │
│                              Frank-Wolfe Projection             │
│                              (find gap from fair prices)        │
│                                       │                         │
│                                       ▼                         │
│                              ClusterOpportunity                 │
└─────────────────────────────────────────────────────────────────┘
```

## Relation Types

The LLM discovers these relation types:

| Type | Constraint | Example |
|------|------------|---------|
| `implies` | P(A) ≤ P(B) | "PA win" → "swing state win" |
| `mutually_exclusive` | Σ P(i) ≤ 1 | "Trump wins" vs "Biden wins" |
| `exactly_one` | Σ P(i) = 1 | All candidates in single-winner race |

## The Math

### The Marginal Polytope

For n binary conditions, valid price vectors live in the marginal polytope M:

```
M = conv({φ(ω) : ω ∈ Ω})
```

Where Ω is the set of possible world states and φ(ω) maps each state to outcome indicators.

### Bregman Projection

To find the optimal trade, project current prices θ onto M using Bregman divergence:

```
D(μ||θ) = Σ μᵢ × ln(μᵢ/θᵢ)
```

The projection μ* is the closest arbitrage-free price vector. The distance represents maximum extractable profit.

### Frank-Wolfe Algorithm

```
1. Start with prices θ
2. Compute gradient ∇D at current point
3. Solve ILP: find vertex v minimizing ⟨∇D, v⟩
4. Step toward v: μ ← μ + α(v - μ)
5. Repeat until gap < ε
```

The ILP oracle (HiGHS solver) finds extreme points of the polytope.

## Implementation

**Location:** `src/core/strategy/combinatorial/`

| File | Purpose |
|------|---------|
| `mod.rs` | `CombinatorialStrategy` implementation |
| `frank_wolfe.rs` | Frank-Wolfe algorithm |
| `bregman.rs` | Divergence calculations |

**Related modules:**

| Module | Purpose |
|--------|---------|
| `core/inference/` | `Inferrer` trait, `LlmInferrer` |
| `core/llm/` | `Llm` trait, Anthropic/OpenAI clients |
| `core/cache/cluster.rs` | `ClusterCache` with TTL |
| `core/service/cluster/` | `ClusterDetectionService` |
| `core/domain/relation.rs` | `Relation`, `RelationKind`, `Cluster` |

## Configuration

```toml
# Enable the strategy
[strategies]
enabled = ["single_condition", "market_rebalancing", "combinatorial"]

[strategies.combinatorial]
enabled = true            # Must be explicitly enabled
max_iterations = 20       # Frank-Wolfe iterations
tolerance = 0.0001        # Convergence threshold
gap_threshold = 0.02      # Minimum gap to trade (2%)

# Enable LLM inference
[inference]
enabled = true
min_confidence = 0.7      # Filter low-confidence relations
ttl_seconds = 3600        # Relation validity (1 hour)
batch_size = 30           # Markets per LLM call

# Configure LLM provider
[llm]
provider = "anthropic"    # or "openai"

[llm.anthropic]
model = "claude-3-5-sonnet-20241022"
temperature = 0.2
max_tokens = 4096

# Enable real-time cluster detection
[cluster_detection]
enabled = true
debounce_ms = 100         # Detection interval
min_gap = 0.02            # Minimum arbitrage gap
max_clusters_per_cycle = 50
```

**Environment variables:**

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
# or
export OPENAI_API_KEY="sk-..."
```

## Limitations

- **LLM costs**: Each inference call costs ~$0.01-0.05
- **Latency**: Initial inference adds 2-5 seconds at startup
- **False positives**: LLM may hallucinate relations (filtered by confidence)
- **Rare opportunities**: Only 0.24% of historical profits came from this

## When to Use

Enable combinatorial strategy when:
- You want to capture cross-market arbitrage
- You're running on mainnet with real capital
- You've validated the LLM is finding real relations

Keep disabled when:
- Testing basic functionality
- Running on testnet (fewer correlated markets)
- Minimizing API costs

## References

- [Arbitrage-Free Combinatorial Market Making via Integer Programming](https://arxiv.org/abs/1606.02825)
- [Unravelling the Probabilistic Forest](https://arxiv.org/abs/2508.03474)
