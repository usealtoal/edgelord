# Custom Strategies

Implement `adapter::strategy::Strategy` to add your own detection algorithm.

## Trait Definition

```rust
pub trait Strategy: Send + Sync {
    fn name(&self) -> &'static str;
    fn applies_to(&self, ctx: &MarketContext) -> bool;
    fn detect(&self, ctx: &DetectionContext) -> Vec<Opportunity>;
}
```

## Example

```rust
use edgelord::adapter::strategy::{Strategy, MarketContext, DetectionContext};
use edgelord::domain::Opportunity;

pub struct MomentumStrategy {
    lookback: usize,
}

impl Strategy for MomentumStrategy {
    fn name(&self) -> &'static str {
        "momentum"
    }

    fn applies_to(&self, ctx: &MarketContext) -> bool {
        ctx.outcome_count == 2  // Binary markets only
    }

    fn detect(&self, ctx: &DetectionContext) -> Vec<Opportunity> {
        // Your detection logic here
        vec![]
    }
}
```

## Registration

Add your strategy to the registry in your fork:

```rust
let registry = StrategyRegistry::builder()
    .strategy(MomentumStrategy::new(20))
    .strategy(SingleConditionStrategy::default())
    .build();
```
