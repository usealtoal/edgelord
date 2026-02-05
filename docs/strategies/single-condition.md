# Single-Condition Strategy

Detects arbitrage in binary markets where YES + NO < $1.

## What It Detects

In a binary prediction market, exactly one outcome wins and pays $1. If you can buy both YES and NO for less than $1 total, you profit the difference.

**Example:**
- YES asking $0.45
- NO asking $0.50
- Total cost: $0.95
- Guaranteed payout: $1.00
- Profit: $0.05 per share

## Intuition

Market makers and traders don't always keep prices perfectly balanced. Temporary imbalances—from large orders, slow updates, or thin liquidity—create windows where the sum drops below $1.

This captured 27% ($10.5M) of historical arbitrage profits. It's the simplest strategy but still profitable because these mispricings happen frequently in less liquid markets.

## The Math

For a binary market with outcomes A and B:

```
Edge = Payout - (Ask_A + Ask_B)
```

Where:
- `Payout` = $1.00 (configurable per exchange)
- `Ask_A` = best ask price for outcome A
- `Ask_B` = best ask price for outcome B

Arbitrage exists when `Edge > 0`.

**Volume** is limited by the smaller side:
```
Volume = min(Size_A, Size_B)
```

**Expected profit**:
```
Profit = Edge × Volume
```

## Worked Example

Market: "Will it rain tomorrow?"

| Outcome | Best Ask | Size |
|---------|----------|------|
| YES | $0.42 | 150 |
| NO | $0.52 | 100 |

Calculations:
- Total cost: $0.42 + $0.52 = $0.94
- Edge: $1.00 - $0.94 = $0.06 (6%)
- Volume: min(150, 100) = 100
- Expected profit: $0.06 × 100 = $6.00

Action: Buy 100 YES at $0.42 and 100 NO at $0.52. One pays $1, profit $6.

## How It's Used

**Location:** `src/core/strategy/condition/single.rs`

**Trait implementation:**
- `applies_to()` returns true for binary markets (2 outcomes)
- `detect()` calls `detect_single_condition()` with market and cache

**Pipeline:**
1. WebSocket update triggers detection
2. Strategy checks if market is binary
3. Fetches best asks from `OrderBookCache`
4. Calculates edge and profit
5. Returns `Opportunity` if thresholds met

**Output:** `Opportunity` struct with two legs (one per outcome).

## Configuration

```toml
[strategies.single_condition]
min_edge = 0.05      # Minimum edge (5%)
min_profit = 0.50    # Minimum profit ($0.50)
```

- **min_edge**: Skip if edge percentage is below this. Higher = fewer but safer trades.
- **min_profit**: Skip if dollar profit is below this. Filters noise from tiny opportunities.

## Limitations

- **Binary only**: Doesn't handle multi-outcome markets (use Market Rebalancing)
- **Best ask only**: Ignores deeper liquidity; large orders may not fill at expected price
- **No partial fills**: Assumes full execution; reality may differ
- **Latency sensitive**: Opportunities disappear fast; sub-second detection matters
