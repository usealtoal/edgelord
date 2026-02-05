# Market Rebalancing Strategy

Detects arbitrage in multi-outcome markets where the sum of all outcomes < $1.

## What It Detects

In a market with N mutually exclusive outcomes, exactly one wins and pays $1. If you can buy all N outcomes for less than $1 total, you profit the difference.

**Example (3-way election):**
- Candidate A: $0.35
- Candidate B: $0.30
- Candidate C: $0.25
- Total cost: $0.90
- Guaranteed payout: $1.00
- Profit: $0.10 per set

## Intuition

Multi-outcome markets are harder to keep balanced. With N outcomes, there are more prices to track and more ways for the sum to drift. Market makers may not cover all outcomes equally, creating persistent imbalances.

This captured 73% ($29M) of historical arbitrage profits—the largest share by far. The extra complexity in pricing creates more opportunities.

## The Math

For a market with outcomes O₁, O₂, ..., Oₙ:

```
Edge = Payout - Σ Ask_i
```

Where:
- `Payout` = $1.00 (configurable)
- `Ask_i` = best ask price for outcome i

Arbitrage exists when `Edge > 0`.

**Volume** is limited by the smallest leg:
```
Volume = min(Size_1, Size_2, ..., Size_n)
```

**Expected profit**:
```
Profit = Edge × Volume
```

## Worked Example

Market: "Who wins the election?" (5 candidates)

| Candidate | Best Ask | Size |
|-----------|----------|------|
| A | $0.25 | 200 |
| B | $0.22 | 150 |
| C | $0.18 | 300 |
| D | $0.15 | 100 |
| E | $0.12 | 250 |

Calculations:
- Total cost: $0.25 + $0.22 + $0.18 + $0.15 + $0.12 = $0.92
- Edge: $1.00 - $0.92 = $0.08 (8%)
- Volume: min(200, 150, 300, 100, 250) = 100
- Expected profit: $0.08 × 100 = $8.00

Action: Buy 100 of each candidate. Winner pays $1, profit $8.

## How It's Used

**Location:** `src/core/strategy/rebalancing/mod.rs`

**Trait implementation:**
- `applies_to()` returns true for 3+ outcomes, up to `max_outcomes`
- `detect()` calls `detect_rebalancing()` with all token IDs

**Pipeline:**
1. WebSocket update triggers detection
2. Strategy checks outcome count (3 to max_outcomes)
3. Fetches best asks for all outcomes from cache
4. Sums prices, calculates edge
5. Returns `Opportunity` with N legs if thresholds met

**Output:** `Opportunity` struct with one leg per outcome.

## Configuration

```toml
[strategies.market_rebalancing]
min_edge = 0.03      # Minimum edge (3%)
min_profit = 1.00    # Minimum profit ($1.00)
max_outcomes = 10    # Skip markets with more outcomes
```

- **min_edge**: Lower than single-condition because multi-outcome is more common
- **min_profit**: Higher because more legs = more execution risk
- **max_outcomes**: Caps complexity; very large markets rarely have good liquidity across all outcomes

## Limitations

- **All-or-nothing**: Must buy all outcomes; partial coverage isn't arbitrage
- **Liquidity fragmentation**: Volume limited by thinnest outcome
- **Execution complexity**: N orders vs 2 for binary; more can go wrong
- **Skips binary**: Binary markets handled by Single-Condition (simpler, faster)
