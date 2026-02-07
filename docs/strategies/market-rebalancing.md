# Market Rebalancing Strategy

Market rebalancing detects underpriced outcome baskets in multi-outcome markets.

## Rule

For outcomes `O1..On`:

```text
edge = payout - sum(ask_Oi)
```

Opportunity exists when `edge > 0` and thresholds are satisfied.

## Why It Works

Multi-outcome books are harder to keep synchronized. The probability mass often drifts, especially in less-liquid tails.

## Configuration

```toml
[strategies.market_rebalancing]
min_edge = 0.03
min_profit = 1.00
max_outcomes = 10
```

## Operational Notes

- Captures more complex opportunities than binary checks.
- Introduces more execution legs and greater partial-fill risk.
- `max_outcomes` is an important guardrail for latency and fill reliability.
