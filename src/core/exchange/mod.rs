//! Exchange abstraction layer.
//!
//! ## Adding a New Exchange
//!
//! 1. Create a module under `exchange/<name>/`
//! 2. Implement [`ExchangeConfig`] trait with:
//!    - `name()` - Exchange identifier
//!    - `default_payout()` - Payout amount per share
//!    - `binary_outcome_names()` - Names for Yes/No outcomes
//! 3. The default `parse_markets()` implementation handles most cases
//! 4. Add to [`ExchangeFactory`] for runtime selection
//!
//! ## Example
//!
//! ```ignore
//! struct MyExchangeConfig;
//!
//! impl ExchangeConfig for MyExchangeConfig {
//!     fn name(&self) -> &'static str { "myexchange" }
//!     fn default_payout(&self) -> Decimal { dec!(1.00) }
//!     fn binary_outcome_names(&self) -> (&'static str, &'static str) { ("Yes", "No") }
//! }
//! ```

mod approval;
mod dedup;
mod factory;
mod filter;
pub mod polymarket;
mod reconnecting;
mod scorer;
mod exchange_config;
mod types;

pub use approval::{ApprovalResult, ApprovalStatus, TokenApproval};
pub use dedup::{DedupConfig, DedupStrategy, MessageDeduplicator};
pub use factory::ExchangeFactory;
pub use filter::{MarketFilter, MarketFilterConfig};
pub use reconnecting::ReconnectingDataStream;
pub use scorer::MarketScorer;
pub use exchange_config::ExchangeConfig;
pub use types::{
    ArbitrageExecutor, ExecutionResult, MarketDataStream, MarketEvent, MarketFetcher, MarketInfo,
    OrderExecutor, OrderRequest, OrderSide, OutcomeInfo,
};
