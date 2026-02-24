# Strategy Guide

## What is Prediction Market Arbitrage?

Prediction markets price uncertain events as tradeable contracts. Each contract pays a fixed amount (typically $1) if its associated outcome occurs. In a well-functioning market, the prices of all mutually exclusive outcomes should sum to the payout value—this reflects the fundamental constraint that exactly one outcome must occur.

When prices temporarily deviate from this constraint, arbitrage opportunities emerge. By simultaneously purchasing all outcomes for less than the guaranteed payout, a trader locks in risk-free profit regardless of which outcome actually occurs.

## The Fundamental Arbitrage Condition

For a market with *n* mutually exclusive outcomes O₁, O₂, ..., Oₙ where exactly one must occur:

**No arbitrage condition:**
$$\sum_{i=1}^{n} P(O_i) = \text{payout}$$

**Arbitrage opportunity exists when:**
$$\sum_{i=1}^{n} \text{ask}_i < \text{payout}$$

The **edge** represents the guaranteed profit per unit:
$$\text{edge} = \text{payout} - \sum_{i=1}^{n} \text{ask}_i$$

## Why Do Arbitrage Opportunities Exist?

Prediction markets are not perfectly efficient. Several factors create temporary price dislocations:

1. **Information asymmetry**: News affects different outcomes at different speeds
2. **Liquidity fragmentation**: Order books for different outcomes update independently
3. **Market maker latency**: Automated systems don't instantly rebalance all prices
4. **Behavioral factors**: Traders focus attention on high-probability outcomes, neglecting the tails

These inefficiencies create windows—typically lasting milliseconds to seconds—where prices violate the no-arbitrage condition.

## Available Strategies

Edgelord implements three strategies that detect different types of arbitrage opportunities:

| Strategy | Market Type | Detects |
|----------|-------------|---------|
| [Single-Condition](single-condition.md) | Binary (YES/NO) | ask_YES + ask_NO < 1 |
| [Market Rebalancing](market-rebalancing.md) | Multi-outcome (3+) | Σ ask_i < payout |
| [Combinatorial](combinatorial.md) | Related market clusters | Cross-market constraint violations |

## Strategy Selection

**Start simple**: Enable `single_condition` and `market_rebalancing` first. These strategies have straightforward detection logic and well-understood risk profiles.

**Graduate to combinatorial**: The combinatorial strategy requires additional infrastructure (inference engine, cluster detection) and careful tuning. Add it once baseline strategies are operationally stable.

## Shared Risk Considerations

All strategies share common execution risks:

- **Edge decay**: The opportunity may disappear between detection and execution
- **Partial fills**: Not all legs may execute, leaving directional exposure
- **Volume constraints**: Tradeable size equals the minimum depth across all legs

## Configuration

Enable strategies in your configuration:

```toml
[strategies]
enabled = ["single_condition", "market_rebalancing"]
```

Each strategy has its own configuration section for fine-tuning detection thresholds.
