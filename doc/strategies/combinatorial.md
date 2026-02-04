# Combinatorial Strategy

Detects arbitrage across correlated markets using Frank-Wolfe optimization and integer programming.

## What It Detects

Some markets have logical dependencies. If Market A implies Market B, their prices must satisfy certain constraints. Violations of these constraints create arbitrage.

**Example:**
- Market A: "Trump wins Pennsylvania" — YES at $0.48
- Market B: "Republicans win PA by 5+ points" — YES at $0.32

If Republicans win by 5+ points, Trump must win PA. So P(B) ≤ P(A). But $0.32 > some implied constraint based on A's price—this can be exploited.

## Intuition

Individual markets may be priced correctly, but the joint probability distribution can still be inconsistent. The "arbitrage-free" region forms a polytope in price space. Prices outside this polytope can be profitably traded back onto it.

This captured only 0.24% ($95K) of historical profits. The math is sophisticated, but the opportunities are rare and require knowing which markets are correlated.

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

For LMSR (Polymarket's market maker), this is KL-divergence.

The projection μ* is the closest arbitrage-free price vector. The distance D(μ*||θ) represents maximum extractable profit.

### Frank-Wolfe Algorithm

Direct projection is intractable for large markets. Frank-Wolfe iteratively approaches the solution:

```
1. Start with prices θ
2. Compute gradient ∇D at current point
3. Solve ILP: find vertex v minimizing ⟨∇D, v⟩
4. Step toward v: μ ← μ + α(v - μ)
5. Repeat until gap < ε
```

The ILP oracle finds extreme points of the polytope. HiGHS solves these subproblems.

**Convergence:** Typically 50-150 iterations for markets with thousands of conditions.

## How It's Used

**Location:** `src/core/strategy/combinatorial/`

**Components:**
- `mod.rs` — Strategy implementation
- `frank_wolfe.rs` — Frank-Wolfe algorithm
- `bregman.rs` — Divergence calculations

**Current status:** Infrastructure is implemented, but requires:
1. Dependency detection (which markets are correlated)
2. Constraint encoding (logical rules as ILP constraints)
3. Multi-market state aggregation

The research paper used LLM-assisted dependency detection. This is not yet implemented.

**Trait implementation:**
- `applies_to()` returns true only if market has known dependencies
- `detect()` currently returns empty (awaiting dependency system)

## Configuration

```toml
[strategies.combinatorial]
enabled = false           # Disabled by default
max_iterations = 20       # Frank-Wolfe iterations
tolerance = 0.0001        # Convergence threshold
gap_threshold = 0.02      # Minimum gap to trade
```

Disabled by default because it requires dependency configuration that doesn't exist yet.

## Limitations

- **Requires dependency knowledge**: Must know which markets are correlated
- **Computationally expensive**: ILP solving on hot path
- **Rare opportunities**: Only 0.24% of historical profits
- **Not fully implemented**: Dependency detection is future work

## References

- [Arbitrage-Free Combinatorial Market Making via Integer Programming](https://arxiv.org/abs/1606.02825)
- [Unravelling the Probabilistic Forest](https://arxiv.org/abs/2508.03474)
