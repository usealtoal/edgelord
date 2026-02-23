# Custom Strategies

Implement `port::inbound::strategy::Strategy` to add your own detection algorithm.

## Strategy Contract

```rust
pub trait Strategy: Send + Sync {
    fn name(&self) -> &'static str;
    fn applies_to(&self, ctx: &MarketContext) -> bool;
    fn detect(&self, ctx: &dyn DetectionContext) -> Vec<Opportunity>;
}
```

Primary types are defined in `src/port/inbound/strategy.rs`:
- `Strategy`
- `MarketContext`
- `DetectionContext`

## Example

```rust
use edgelord::domain::opportunity::Opportunity;
use edgelord::port::inbound::strategy::{DetectionContext, MarketContext, Strategy};

pub struct MomentumStrategy;

impl Strategy for MomentumStrategy {
    fn name(&self) -> &'static str {
        "momentum"
    }

    fn applies_to(&self, ctx: &MarketContext) -> bool {
        ctx.is_binary()
    }

    fn detect(&self, _ctx: &dyn DetectionContext) -> Vec<Opportunity> {
        vec![]
    }
}
```

## Where to Place It

- Add strategy implementation in `src/application/strategy/`.
- Keep strategy-specific config and logic in the same module.
- Keep infrastructure and exchange code out of strategy modules.

## Registration

`StrategyRegistry` lives in `src/application/strategy/registry.rs`.

To wire a custom strategy in your fork:

1. Add a config entry in `src/infrastructure/config/strategy.rs`.
2. Extend `StrategyRegistryBuilder` in `src/application/strategy/registry.rs`.
3. Register it in `build_strategy_registry` in `src/infrastructure/bootstrap.rs`.

This keeps strategy enablement fully config-driven while preserving hexagonal boundaries.
