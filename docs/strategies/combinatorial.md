# Combinatorial Strategy

Combinatorial strategy detects cross-market arbitrage based on inferred logical relations.

## Pipeline

1. Relation inference proposes constraints between markets.
2. Cluster detection watches related markets for meaningful updates.
3. Optimizer/solver path evaluates constraint violations and potential edge.

## Typical Relation Types

- `implies`
- `mutually_exclusive`
- `exactly_one`

## Configuration

```toml
[strategies.combinatorial]
enabled = true
max_iterations = 20
tolerance = 0.0001
gap_threshold = 0.02

[inference]
enabled = true
min_confidence = 0.7
ttl_seconds = 3600

[cluster_detection]
enabled = true
min_gap = 0.02
```

## Trade-Offs

- More expressive opportunity space.
- Higher implementation and operational complexity.
- Requires careful confidence tuning and observability.

Use combinatorial after baseline strategies are stable in your environment.
