# Polymarket Arbitrage Research

> Source: "Unravelling the Probabilistic Forest: Arbitrage in Prediction Markets" (arXiv:2508.03474)
> Theory: "Arbitrage-Free Combinatorial Market Making via Integer Programming" (arXiv:1606.02825)

## Executive Summary

Sophisticated traders extracted **$40 million** in guaranteed arbitrage profits from Polymarket (April 2024 - April 2025). The top trader alone made **$2,009,631.76** from 4,049 trades (~$496/trade average).

This isn't gambling. It's solving optimization problems.

---

## The Three Types of Arbitrage

### 1. Single-Condition Arbitrage
**What:** YES + NO ≠ $1.00

```
Example:
  YES: $0.62
  NO:  $0.33
  Sum: $0.95  ← Buy both for $0.95, guaranteed $1.00 payout
  Profit: $0.05 per share
```

**Prevalence:** 41% of conditions (7,051 out of 17,218)
**Median mispricing:** $0.60 (should be $1.00) — 40% off!

### 2. Market Rebalancing Arbitrage
**What:** Sum of all outcome prices ≠ $1.00

```
Example (3-way market):
  Candidate A: $0.40
  Candidate B: $0.35
  Candidate C: $0.20
  Sum: $0.95  ← Buy all for $0.95, one pays $1.00
  Profit: $0.05 per set
```

**Prevalence:** 42% of multi-condition markets

### 3. Combinatorial Arbitrage (The Hard One)
**What:** Logical dependencies between markets create hidden arbitrage

```
Example:
  Market A: "Trump wins Pennsylvania" — YES $0.48
  Market B: "Republicans win PA by 5+ points" — YES $0.32

  Dependency: If Republicans win by 5+, Trump MUST win PA

  This creates constraints that, when violated, allow profit extraction
  across both markets simultaneously.
```

**Prevalence:** 13 confirmed exploitable pairs in 2024 election

---

## The Math

### LMSR Cost Function
Polymarket uses the Logarithmic Market Scoring Rule:

```
Cost function:     C(q) = b × log(Σ exp(qᵢ/b))
Marginal price:    Pᵢ = exp(qᵢ/b) / Σₖ exp(qₖ/b)
```

Where:
- `q` = vector of net quantities sold
- `b` = liquidity parameter (controls price sensitivity)
- Prices always sum to 1.0 in a properly functioning market

### The Marginal Polytope Problem

For `n` conditions, there are `2^n` possible price combinations but only `n` valid outcomes.

**Valid payoff vectors:** `Z = {φ(ω) : ω ∈ Ω}`
**Arbitrage-free region:** `M = conv(Z)` (convex hull)

Prices outside `M` are exploitable.

**The problem:** NCAA tournament had 63 games = `2^63` possible outcomes. Can't enumerate.

**The solution:** Integer programming constraints

```
Z = {z ∈ {0,1}^I : A^T × z ≥ b}
```

Three linear constraints can replace 16,384 brute force checks.

### Bregman Projection

To find the optimal trade, project current prices onto the arbitrage-free manifold:

```
D(μ||θ) = R(μ) + C(θ) - θ·μ
```

For LMSR, `R(μ)` is negative entropy, making `D` the KL-divergence.

**Maximum guaranteed profit = D(μ*||θ)** where μ* is the Bregman projection.

### Frank-Wolfe Algorithm

Makes the projection computationally tractable:

```
1. Start with small set of known vertices Z₀
2. For iteration t:
   a. Solve convex optimization over conv(Z_{t-1})
   b. Find new descent vertex via IP solver
   c. Add to active set
   d. Check convergence gap
   e. Stop if gap ≤ ε
```

**Convergence:** 50-150 iterations for markets with thousands of conditions
**IP Solver:** Gurobi (10-100x faster than open source alternatives)

---

## Execution Reality

### The Speed Hierarchy

```
Retail trader:
  Polymarket API call:        ~50ms
  Matching engine:            ~100ms
  Polygon block time:         ~2,000ms
  Block propagation:          ~500ms
  Total:                      ~2,650ms

Sophisticated system:
  WebSocket price feed:       <5ms
  Decision computation:       <10ms
  Direct RPC submission:      ~15ms
  Parallel execution:         ~10ms
  Polygon block time:         ~2,000ms (unavoidable)
  Total:                      ~2,040ms
```

**The edge:** ~600ms faster. Submit all legs in same block.

### Why Copytrading Fails

```
Block N-1: Fast system detects, submits 4 txs in 30ms
Block N:   All transactions confirm, arbitrage captured
Block N+1: You see it, copy trade at worse price
```

You're providing exit liquidity, not arbitraging.

### VWAP Analysis

Don't assume instant fills. Calculate Volume-Weighted Average Price:

```
VWAP = Σ(price_i × volume_i) / Σ(volume_i)
```

The research used 950-block windows (~1 hour) to group related trades.

---

## Profit Breakdown (April 2024 - April 2025)

| Type | Profit |
|------|--------|
| Single condition (buy both < $1) | $5,899,287 |
| Single condition (sell both > $1) | $4,682,075 |
| Market rebalancing (buy all YES) | $11,092,286 |
| Market rebalancing (sell all YES) | $612,189 |
| Market rebalancing (buy all NO) | $17,307,114 |
| Combinatorial | $95,634 |
| **Total** | **$39,688,585** |

Top 10 extractors: $8,127,849 (20.5% of total)

---

## Key Insights

1. **Simple arbitrage is most profitable** — Don't overcomplicate
2. **Speed matters but isn't everything** — 2-second block time is the real constraint
3. **Liquidity limits profit** — Can only extract up to order book depth
4. **$0.05 minimum threshold** — Smaller edges get eaten by execution risk
5. **Pre-computation is key** — Don't run heavy optimization on hot path

---

## References

- [Unravelling the Probabilistic Forest (arXiv:2508.03474)](https://arxiv.org/abs/2508.03474)
- [Arbitrage-Free Combinatorial Market Making (arXiv:1606.02825)](https://arxiv.org/abs/1606.02825)
- [LMSR Primer (Gnosis)](https://gnosis-pm-js.readthedocs.io/en/v1.3.0/lmsr-primer.html)
- [Frank-Wolfe Algorithm (Wikipedia)](https://en.wikipedia.org/wiki/Frank%E2%80%93Wolfe_algorithm)
