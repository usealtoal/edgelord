# Market Rebalancing Strategy

## Overview

The market rebalancing strategy detects arbitrage in **multi-outcome prediction markets**—markets with three or more mutually exclusive outcomes. When the sum of all outcome prices falls below the guaranteed payout, buying every outcome locks in a risk-free profit since exactly one must pay out.

This strategy captures the majority of arbitrage profits (~73% historically) because multi-outcome markets are inherently harder to keep synchronized.

## The Mathematics of Multi-Outcome Arbitrage

### The Fundamental Constraint

Consider a market with *n* mutually exclusive outcomes O₁, O₂, ..., Oₙ. Exactly one outcome will occur. If the market pays $1 to holders of the winning outcome:

$$\sum_{i=1}^{n} P(O_i) = 1$$

In equilibrium, the sum of all outcome prices should equal the payout.

### The Arbitrage Condition

An arbitrage opportunity exists when:

$$\sum_{i=1}^{n} \text{ask}_i < \text{payout}$$

By purchasing one unit of every outcome, you spend less than the payout but are guaranteed to receive exactly the payout when one outcome resolves.

### Calculating Edge and Profit

**Total cost**:
$$C = \sum_{i=1}^{n} \text{ask}_i$$

**Edge** (as a fraction of payout):
$$\text{edge} = \frac{\text{payout} - C}{\text{payout}}$$

**Tradeable volume** (constrained by the thinnest leg):
$$V = \min_{i \in \{1..n\}} \text{size}_i$$

**Profit**:
$$\text{profit} = (\text{payout} - C) \times V$$

### Worked Example

Consider a 4-outcome market (e.g., "Which candidate wins the primary?"):

| Outcome | Ask Price | Available Size |
|---------|-----------|----------------|
| Candidate A | $0.35 | $500 |
| Candidate B | $0.28 | $200 |
| Candidate C | $0.18 | $800 |
| Candidate D | $0.15 | $150 |

**Total cost**: $0.35 + $0.28 + $0.18 + $0.15 = $0.96

**Edge**: ($1.00 - $0.96) / $1.00 = 4%

**Tradeable volume**: min($500, $200, $800, $150) = $150

**Profit**: $0.04 × $150 = $6.00 guaranteed

No matter which candidate wins, you receive $150 (150 units × $1 payout) while having paid only $144.

## Why Multi-Outcome Markets Misprice

Multi-outcome markets exhibit more frequent and larger arbitrage opportunities than binary markets:

### 1. Coordination Complexity

With *n* outcomes, there are *n* independent order books. Keeping them synchronized requires *n* times the market-making attention. As *n* grows, coordination failures become more likely.

### 2. Tail Outcome Neglect

Traders focus on high-probability outcomes. If Candidate A is leading at 35%, market participants actively trade that contract. But Candidate D at 15% receives less attention, allowing its price to drift without correction.

### 3. Correlated Information Flow

News often affects a subset of outcomes. A scandal involving Candidate A might cause traders to sell A and buy B, but the immediate repricing doesn't necessarily adjust C and D proportionally. This creates temporary sum violations.

### 4. Liquidity Fragmentation

Capital spreads across more outcomes, making each individual book thinner. Thin books are more susceptible to price dislocations from even small order flow.

## Probability Drift Over Time

Consider how a 5-outcome market might evolve:

**t=0** (equilibrium):
$$A: 0.40 + B: 0.25 + C: 0.20 + D: 0.10 + E: 0.05 = 1.00$$

**t=1** (news breaks favoring A):
$$A: 0.52 + B: 0.22 + C: 0.18 + D: 0.08 + E: 0.05 = 1.05$$

Market is temporarily overpriced (no arbitrage on the buy side, but potential on the sell side if short-selling is available).

**t=2** (overcorrection):
$$A: 0.48 + B: 0.20 + C: 0.16 + D: 0.07 + E: 0.04 = 0.95$$

Now the market is underpriced by 5%. This is the arbitrage window—buy all outcomes for $0.95, receive $1.00 guaranteed.

These oscillations create recurring opportunities, especially around news events.

## Detection Logic

The strategy evaluates each multi-outcome market by:

1. Skipping markets with fewer than 3 outcomes (handled by single-condition)
2. Skipping markets exceeding max_outcomes (execution risk too high)
3. For each outcome, fetching best ask price and size
4. Computing total_cost = Σ ask_i
5. Computing edge = (payout - total_cost) / payout
6. Computing volume = min(size_i) across all outcomes
7. If edge ≥ min_edge and profit ≥ min_profit, emit an opportunity

## Volume Constraints

The effective tradeable volume equals the minimum depth across all legs:

$$V = \min(V_1, V_2, ..., V_n)$$

This creates a characteristic pattern: the thinnest leg becomes the bottleneck. In the worked example above, Candidate D's $150 depth limits the entire opportunity, even though other outcomes have much more liquidity available.

Sophisticated arbitrageurs sometimes provide liquidity to thin outcomes specifically to increase their tradeable volume on rebalancing opportunities.

## Risk Analysis

### Partial Fill Risk

With more legs, the probability of complete execution decreases:

$$P(\text{all filled}) = P(\text{single fill})^n$$

If each leg has a 95% fill probability:
- 2 legs: 0.95² = 90.25%
- 4 legs: 0.95⁴ = 81.45%
- 8 legs: 0.95⁸ = 66.34%

Partial fills leave directional exposure—you own some outcomes but not all, converting a risk-free trade into a speculative position.

### Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Partial fills | Atomic execution where possible, conservative sizing |
| Stale prices | Per-leg freshness checks before execution |
| Excessive legs | max_outcomes limit prevents high-complexity trades |
| Thin liquidity | Minimum volume thresholds |

## Configuration

```toml
[strategies.market_rebalancing]
min_edge = 0.03        # Minimum 3% edge
min_profit = 1.00      # Minimum $1.00 expected profit
max_outcomes = 10      # Skip markets with >10 outcomes
```

### Tuning Guidelines

| Approach | min_edge | min_profit | max_outcomes |
|----------|----------|------------|--------------|
| Conservative | 5% | $2.00 | 6 |
| Aggressive | 2% | $0.50 | 15 |

Conservative settings prioritize execution reliability over opportunity frequency. Aggressive settings capture more opportunities but require robust execution infrastructure and higher risk tolerance.

## Performance Characteristics

- **Detection complexity**: O(n) where n = number of outcomes
- **Typical edge**: 1-3% when opportunities exist
- **Historical contribution**: ~73% of total arbitrage profits

The market rebalancing strategy dominates profit contribution because multi-outcome markets offer more frequent and larger mispricings than binary markets.

## Comparison with Single-Condition

| Dimension | Single-Condition | Market Rebalancing |
|-----------|------------------|-------------------|
| Market type | Binary (2 outcomes) | Multi-outcome (3+) |
| Detection complexity | O(1) | O(n) |
| Execution complexity | Lower (2 legs) | Higher (n legs) |
| Profit contribution | ~15% | ~73% |
| Opportunity frequency | Higher | Lower |
| Edge per opportunity | Smaller | Larger |

The strategies are complementary—single-condition captures frequent small opportunities in binary markets, while market rebalancing captures less frequent but larger opportunities in multi-outcome markets.
