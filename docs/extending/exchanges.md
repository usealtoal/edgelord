# Custom Exchanges

Implement exchange traits to add support for new prediction markets.

## Required Traits

### MarketFetcher
Fetch available markets from the exchange.

```rust
#[async_trait]
pub trait MarketFetcher: Send + Sync {
    async fn fetch_markets(&self) -> Result<Vec<Market>>;
}
```

### MarketDataStream
Stream real-time order book updates.

```rust
#[async_trait]
pub trait MarketDataStream: Send {
    async fn subscribe(&mut self, token_ids: &[TokenId]) -> Result<()>;
    async fn next_event(&mut self) -> Option<MarketEvent>;
}
```

### ArbitrageExecutor
Execute multi-leg arbitrage opportunities.

```rust
#[async_trait]
pub trait ArbitrageExecutor: Send + Sync {
    async fn execute(&self, opp: &Opportunity) -> Result<ArbitrageExecutionResult>;
}
```

## Example Structure

```
src/adapters/kalshi/
├── mod.rs           # Module exports
├── client.rs        # HTTP client
├── websocket.rs     # MarketDataStream impl
└── executor.rs      # ArbitrageExecutor impl
```
