# Strategy Guide

Edgelord supports multiple complementary arbitrage strategies. Each strategy emits opportunities into a shared risk and execution pipeline.

## Available Strategies

| Strategy | Market Scope | Signal |
|---|---|---|
| [Single-Condition](single-condition.md) | Binary markets | `YES + NO < payout` |
| [Market Rebalancing](market-rebalancing.md) | Multi-outcome markets | `sum(outcomes) < payout` |
| [Combinatorial](combinatorial.md) | Related market clusters | Cross-market constraint violations |

## Selection Guidance

- Start with `single_condition` + `market_rebalancing` for operational simplicity.
- Add `combinatorial` once inference and cluster detection are tuned for your environment.

## Common Risk Considerations

- Opportunity edge can collapse between detection and execution.
- Effective size is constrained by the thinnest leg.
- Higher-leg-count opportunities increase partial-fill risk.

## Enabling Strategies

```toml
[strategies]
enabled = ["single_condition", "market_rebalancing"]
```

For combinatorial mode, also enable and configure:

```toml
[strategies.combinatorial]
enabled = true

[inference]
enabled = true

[cluster_detection]
enabled = true
```
