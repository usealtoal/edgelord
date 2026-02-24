# Combinatorial Strategy

## Overview

The combinatorial strategy detects arbitrage across **related but separate markets** by recognizing logical relationships between them. Unlike single-condition and market rebalancing—which operate within a single market—combinatorial arbitrage exploits constraint violations that span multiple markets.

This is the most sophisticated strategy, requiring inference of relationships that aren't explicitly encoded in market structure.

## The Conceptual Foundation

### Beyond Single-Market Constraints

Single-condition and market rebalancing enforce a simple constraint: outcomes within a market must sum to the payout. But prediction markets often contain implicit cross-market relationships that create additional constraints.

Consider three separate markets:
- Market A: "Will candidate X win the primary?"
- Market B: "Will candidate X win the general election?"
- Market C: "Will candidate X become president?"

These markets are logically related:

$$P(C) \leq P(B) \leq P(A)$$

You cannot become president without winning the general, and you cannot win the general without winning the primary. If market prices violate these inequalities, arbitrage exists.

### The General Form

For markets with inferred relationship R:

$$\text{constraint}(P_1, P_2, ..., P_k | R) = 0$$

When prices violate this constraint:

$$\text{constraint}(\text{ask}_1, \text{ask}_2, ..., \text{ask}_k | R) \neq 0$$

The **gap** represents the magnitude of violation and indicates potential arbitrage.

## Relation Types

The combinatorial strategy recognizes several fundamental logical relationships:

### Implication (A → B)

If A occurs, B must also occur. Therefore:

$$P(A) \leq P(B)$$

**Arbitrage condition**: ask_A > ask_B

If A implies B but A costs more than B, sell A and buy B. If A happens, both contracts pay out (net zero). If A doesn't happen, you keep the premium from selling A.

**Example**: "Democrats win Texas" → "Democrats win presidency"
If Texas goes blue, no modern election has the Democrat losing the presidency. Yet these markets may price the Texas contract higher due to isolated trading.

### Mutual Exclusivity (A ⊕ B)

A and B cannot both occur. Therefore:

$$P(A) + P(B) \leq 1$$

**Arbitrage condition**: ask_A + ask_B > 1 (sell side) or bid_A + bid_B < 1 (buy side, if both losing means a third outcome won)

**Example**: "Candidate X wins" and "Candidate Y wins" in a two-person race.

### Exactly One (A₁ ⊕ A₂ ⊕ ... ⊕ Aₙ)

Exactly one of the outcomes must occur. This generalizes mutual exclusivity:

$$\sum_{i=1}^{n} P(A_i) = 1$$

This is the same constraint as market rebalancing, but applied across separate markets rather than within a single market.

**Example**: Separate markets for each candidate winning an election, rather than a single multi-outcome market.

### Conditional Independence

Given some condition C:

$$P(A \cap B | C) = P(A|C) \times P(B|C)$$

This creates tradeable constraints when conditional markets exist alongside joint outcome markets.

## The Detection Pipeline

Combinatorial arbitrage requires three components working together:

### 1. Relation Inference

The inference engine proposes relationships between markets based on:
- Textual analysis of market questions
- Historical price correlations
- Semantic similarity of outcomes
- Known causal or logical structures

Each inferred relation has a **confidence score** reflecting certainty about the relationship.

### 2. Cluster Detection

Related markets form clusters. The cluster detector:
- Groups markets by inferred relationships
- Monitors clusters for price updates
- Triggers evaluation when prices change materially

### 3. Constraint Evaluation

When a cluster updates, the evaluator:
- Checks current prices against relationship constraints
- Computes the gap (magnitude of violation)
- Determines tradeable positions and expected profit
- Emits opportunities exceeding configured thresholds

## Mathematical Framework

### The Constraint Satisfaction Problem

For a cluster of *k* related markets with relationship R, define:

**Constraint function**:
$$f(p_1, p_2, ..., p_k | R) = 0$$

**Gap** (violation magnitude):
$$\text{gap} = |f(\text{price}_1, \text{price}_2, ..., \text{price}_k | R)|$$

**Edge** (accounting for transaction costs):
$$\text{edge} = \text{gap} - \text{costs}$$

### Optimization

For complex relationship graphs, finding the optimal arbitrage trade requires solving a constrained optimization problem:

$$\max_{\mathbf{x}} \sum_i x_i \cdot \text{edge}_i$$

Subject to:
$$\sum_i |x_i| \cdot \text{cost}_i \leq \text{budget}$$
$$x_i \in [-\text{limit}_i, +\text{limit}_i]$$

Where x_i represents position size in market i (positive = buy, negative = sell).

The strategy uses iterative solvers with configurable convergence parameters.

## Confidence and Risk

### Inference Uncertainty

Unlike single-condition and market rebalancing—where the arbitrage constraint is definitionally true—combinatorial arbitrage depends on *inferred* relationships that may be wrong.

If the inference engine incorrectly believes A → B, and you trade on this assumption, you may lose money when A occurs but B doesn't.

**Confidence thresholds** filter out uncertain relationships:

$$\text{confidence}(R) \geq \text{min-confidence}$$

Only relationships meeting this threshold generate trading signals.

### Confidence Decay

Inferred relationships may become stale:

$$\text{effective-confidence}(t) = \text{initial-confidence} \times e^{-\lambda(t - t_0)}$$

Relationships are periodically re-evaluated and may expire if not refreshed.

## Configuration

```toml
[strategies.combinatorial]
enabled = true
max_iterations = 20      # Solver iteration limit
tolerance = 0.0001       # Convergence tolerance
gap_threshold = 0.02     # Minimum 2% gap to consider

[inference]
enabled = true
min_confidence = 0.7     # 70% confidence minimum
ttl_seconds = 3600       # Re-evaluate relations hourly

[cluster_detection]
enabled = true
min_gap = 0.02           # Minimum gap to trigger evaluation
```

### Tuning Guidelines

| Parameter | Conservative | Aggressive |
|-----------|--------------|------------|
| min_confidence | 0.85 | 0.60 |
| gap_threshold | 3% | 1% |
| ttl_seconds | 1800 | 7200 |

Conservative settings require high-confidence relationships and larger gaps, reducing false positives but missing opportunities. Aggressive settings capture more opportunities but require more robust inference and risk management.

## Operational Considerations

### Complexity

The combinatorial strategy is significantly more complex than baseline strategies:

- Requires NLP/semantic analysis for relation inference
- Needs persistent storage for relationship graphs
- Demands careful confidence calibration
- Has more failure modes and edge cases

### Deployment Recommendation

Deploy combinatorial **after** single-condition and market rebalancing are operationally stable:

1. Run baseline strategies, validate execution pipeline
2. Enable inference engine, validate relationship accuracy
3. Enable cluster detection, validate triggering behavior
4. Enable combinatorial trading with conservative settings
5. Gradually relax thresholds as confidence grows

### Observability

Monitor:
- Inferred relationship accuracy (backtested against resolutions)
- Confidence distribution of active relationships
- Gap distribution and capture rate
- False positive rate (apparent arbitrage that loses money)

## Comparison with Baseline Strategies

| Dimension | Single-Condition | Market Rebalancing | Combinatorial |
|-----------|------------------|-------------------|---------------|
| Scope | Within binary market | Within multi-outcome market | Across related markets |
| Constraint source | Definitional | Definitional | Inferred |
| Inference required | No | No | Yes |
| Implementation complexity | Low | Medium | High |
| Confidence | 100% (by definition) | 100% (by definition) | Variable |
| Typical edge | 2-5% | 1-3% | 1-5% |
| Historical contribution | ~15% | ~73% | ~12% |

The combinatorial strategy completes the arbitrage detection suite by capturing opportunities invisible to single-market analysis.
