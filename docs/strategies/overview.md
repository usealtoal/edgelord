# Strategy System

The strategy system provides pluggable arbitrage detection. Each strategy implements the `Strategy` trait and registers with the `StrategyRegistry`.

## How It Works

```
┌─────────────────────────────────────────────────────────────────┐
│                    ORDER BOOK UPDATE                            │
└─────────────────────────────────────────────────────────────────┘
                              │
          ┌───────────────────┼───────────────────┐
          │                   │                   │
          ▼                   ▼                   ▼
   ┌─────────────┐    ┌─────────────┐    ┌─────────────────┐
   │   Single    │    │  Market     │    │  Cluster        │
   │  Condition  │    │ Rebalancing │    │  Detection      │
   │             │    │             │    │  Service        │
   │  YES+NO<$1? │    │  Σ < $1?    │    │                 │
   └──────┬──────┘    └──────┬──────┘    │  Frank-Wolfe    │
          │                   │          │  on dirty       │
          │                   │          │  clusters       │
          │                   │          └────────┬────────┘
          │                   │                   │
          └───────────────────┼───────────────────┘
                              │
                              ▼
                     ┌─────────────────┐
                     │  Opportunities  │
                     │  → Risk Manager │
                     │  → Executor     │
                     └─────────────────┘
```

## Available Strategies

| Strategy | Market Type | Complexity | Historical Share |
|----------|------------|------------|------------------|
| [Single-Condition](single-condition.md) | Binary (2 outcomes) | O(1) | 27% ($10.5M) |
| [Market Rebalancing](market-rebalancing.md) | Multi-outcome (3+) | O(n) | 73% ($29M) |
| [Combinatorial](combinatorial.md) | Correlated markets | O(ILP) | <1% ($95K) |

### Single-Condition

Checks if YES + NO prices sum to less than $1 in binary markets. Simplest and fastest.

### Market Rebalancing

Checks if all outcome prices sum to less than $1 in multi-outcome markets. Handles 3+ outcomes.

### Combinatorial

Uses LLM-powered inference to discover market relations, then Frank-Wolfe optimization to find cross-market arbitrage. Most sophisticated.

**Requirements:**
- LLM API key (Anthropic or OpenAI)
- `inference.enabled = true`
- `cluster_detection.enabled = true`
- `strategies.combinatorial.enabled = true`

## The Strategy Trait

```rust
pub trait Strategy: Send + Sync {
    /// Unique name for logging and config
    fn name(&self) -> &'static str;

    /// Should this strategy run for this market?
    fn applies_to(&self, ctx: &MarketContext) -> bool;

    /// Find opportunities in current market state
    fn detect(&self, ctx: &DetectionContext) -> Vec<Opportunity>;

    /// Optional: warm-start from previous run
    fn warm_start(&mut self, previous: &DetectionResult) {}
}
```

## Configuration

Enable strategies in `config.toml`:

```toml
[strategies]
enabled = ["single_condition", "market_rebalancing", "combinatorial"]

[strategies.single_condition]
min_edge = 0.05
min_profit = 0.50

[strategies.market_rebalancing]
min_edge = 0.03
min_profit = 1.00

[strategies.combinatorial]
enabled = true
gap_threshold = 0.02
```

For combinatorial, also configure:

```toml
[inference]
enabled = true

[cluster_detection]
enabled = true

[llm]
provider = "anthropic"
```

See individual strategy docs for all parameters.

## Adding a New Strategy

1. Create module in `src/core/strategy/<category>/`
2. Implement `Strategy` trait
3. Add config struct with `#[derive(Deserialize)]`
4. Register in `StrategyRegistryBuilder`
5. Document in `docs/strategies/`

## Testing Strategies

Each strategy has unit tests in its module. Integration tests live in `tests/`:

```bash
cargo test --test inference_integration
cargo test --test cluster_detection_tests
```
