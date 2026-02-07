# Single-Condition Strategy

Single-condition detects binary mispricing in one market at a time.

## Rule

For a binary market with outcomes `A` and `B`:

```text
edge = payout - (ask_A + ask_B)
```

Opportunity exists when `edge > 0` and configured thresholds are met.

## Why It Works

Binary books can temporarily drift out of parity under bursty flow and thin liquidity, creating short-lived low-complexity opportunities.

## Configuration

```toml
[strategies.single_condition]
min_edge = 0.05
min_profit = 0.50
```

## Operational Notes

- Fastest strategy in the stack.
- Best for early production hardening.
- Sensitive to stale books and latency spikes.
