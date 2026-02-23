# Custom Exchanges

Implement exchange traits to add support for new prediction markets.

## Required Traits

### MarketFetcher
Fetch available markets from the exchange.

```rust
#[async_trait]
pub trait MarketFetcher: Send + Sync {
    async fn get_markets(&self, limit: usize) -> Result<Vec<MarketInfo>, Error>;
    fn exchange_name(&self) -> &'static str;
}
```

### MarketDataStream
Stream real-time order book updates.

```rust
#[async_trait]
pub trait MarketDataStream: Send {
    async fn connect(&mut self) -> Result<(), Error>;
    async fn subscribe(&mut self, token_ids: &[TokenId]) -> Result<()>;
    async fn next_event(&mut self) -> Option<MarketEvent>;
    fn exchange_name(&self) -> &'static str;
}
```

### ArbitrageExecutor
Execute multi-leg arbitrage opportunities.

```rust
#[async_trait]
pub trait ArbitrageExecutor: Send + Sync {
    async fn execute_arbitrage(&self, opportunity: &Opportunity) -> Result<TradeResult, Error>;
    async fn cancel(&self, order_id: &OrderId) -> Result<(), Error>;
    fn exchange_name(&self) -> &'static str;
}
```

### MarketParser
Parse exchange market payloads into domain markets.

```rust
pub trait MarketParser: Send + Sync {
    fn name(&self) -> &'static str;
    fn default_payout(&self) -> Decimal;
    fn binary_outcome_names(&self) -> (&'static str, &'static str);
    fn parse_markets(&self, market_infos: &[MarketInfo]) -> Vec<Market>;
}
```

## Example Structure

```
src/adapter/outbound/kalshi/
├── mod.rs           # Module exports
├── client.rs        # HTTP client
├── stream.rs        # MarketDataStream impl
├── executor.rs      # ArbitrageExecutor impl
└── market.rs        # MarketParser impl
```
