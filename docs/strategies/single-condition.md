# Single-Condition Strategy

## Overview

The single-condition strategy detects arbitrage in **binary prediction markets**—markets with exactly two mutually exclusive outcomes (YES and NO). When the cost of buying both outcomes falls below the guaranteed payout, purchasing both locks in a risk-free profit.

## The Mathematics of Binary Market Arbitrage

### The Fundamental Constraint

A binary market has two outcomes: YES and NO. Exactly one must occur. If the market pays $1 to the winning outcome holder:

$$P(\text{YES}) + P(\text{NO}) = 1$$

In an efficient market, prices should reflect this constraint. The ask prices for YES and NO should sum to approximately $1.

### The Arbitrage Condition

An arbitrage opportunity exists when:

$$\text{ask}_{\text{YES}} + \text{ask}_{\text{NO}} < 1$$

If this condition holds, buying one unit of both outcomes costs less than $1, but exactly one will pay out $1—guaranteeing profit.

### Calculating Edge and Profit

The **edge** is the profit margin per dollar of payout:

$$\text{edge} = 1 - (\text{ask}_{\text{YES}} + \text{ask}_{\text{NO}})$$

The **profit** scales with volume:

$$\text{profit} = \text{edge} \times \text{volume}$$

Where volume is constrained by the smaller order book:

$$\text{volume} = \min(\text{size}_{\text{YES}}, \text{size}_{\text{NO}})$$

### Worked Example

Consider a market asking:
- YES: $0.52
- NO: $0.45

**Total cost**: $0.52 + $0.45 = $0.97

**Edge**: $1.00 - $0.97 = $0.03 (3%)

**On $100 volume**: $100 × 0.03 = $3.00 guaranteed profit

Regardless of whether the event occurs (YES wins) or doesn't occur (NO wins), you receive $100 while having paid only $97.

## Why Binary Markets Misprice

Binary markets can temporarily violate the no-arbitrage condition due to:

1. **Asymmetric order flow**: A large buy on YES pushes its price up, but the NO price doesn't immediately adjust downward
2. **Latency gaps**: Market makers for YES and NO may update at different frequencies
3. **Liquidity imbalance**: If one side has thin depth, prices drift more easily
4. **News events**: Rapid repricing creates temporary dislocations as one side reacts faster than the other

These windows are typically brief—milliseconds to seconds—before arbitrageurs or market makers correct the imbalance.

## Detection Logic

The strategy evaluates each binary market by:

1. Fetching the best ask price and size for both YES and NO
2. Computing total cost = ask_YES + ask_NO
3. Computing edge = 1 - total cost
4. If edge ≥ min_edge and expected profit ≥ min_profit, emit an opportunity

## Execution Constraints

### Volume Limitation

You can only capture as much edge as the thinner order book allows:

$$\text{tradeable volume} = \min(\text{depth}_{\text{YES}}, \text{depth}_{\text{NO}})$$

If YES has $500 available at $0.52 but NO only has $100 at $0.45, you can only trade $100 worth.

### Edge Decay

Between detection and execution, the opportunity may shrink or vanish:

$$\text{effective edge} = \text{detected edge} - (\text{latency} \times \text{decay rate})$$

In active markets, decay rates of 0.1-0.5% per 100ms are common. Speed matters.

## Risk Analysis

| Risk | Description | Mitigation |
|------|-------------|------------|
| Partial fill | One leg executes, the other doesn't | Atomic execution, conservative sizing |
| Stale prices | Order book changed since detection | Verify prices immediately before execution |
| Slippage | Large orders move the market | Respect size limits, set slippage tolerance |

## Configuration

```toml
[strategies.single_condition]
min_edge = 0.05      # Minimum 5% edge to consider
min_profit = 0.50    # Minimum $0.50 expected profit
```

### Tuning Guidelines

| Approach | min_edge | min_profit | Trade-off |
|----------|----------|------------|-----------|
| Conservative | 5% | $1.00 | Fewer opportunities, higher confidence |
| Aggressive | 2% | $0.25 | More opportunities, requires faster execution |

Higher thresholds reduce false positives but miss smaller opportunities. Lower thresholds increase volume but demand lower latency to remain profitable.

## Performance Characteristics

- **Detection complexity**: O(1)—pure arithmetic on two prices
- **Typical edge**: 2-5% when opportunities exist
- **Historical contribution**: ~15% of total arbitrage profits

The single-condition strategy captures fewer profits than market rebalancing primarily because binary markets are simpler and more heavily arbitraged.
