//! Exchange abstraction layer.
//!
//! ## Adding a New Exchange
//!
//! 1. Create a module under `exchange/<name>/`
//! 2. Implement [`crate::port::MarketDataStream`] and [`crate::port::ArbitrageExecutor`]
//! 3. Implement [`ExchangeConfig`] trait with:
//!    - `name()` - Exchange identifier
//!    - `default_payout()` - Payout amount per share
//!    - `binary_outcome_names()` - Names for Yes/No outcomes
//! 4. Add to [`ExchangeFactory`] for runtime selection

mod approval;
mod dedup;
mod exchange_config;
mod factory;
mod filter;
mod pool;
mod reconnecting;
mod scorer;

pub use approval::{ApprovalResult, ApprovalStatus, TokenApproval};
pub use dedup::{DedupConfig, DedupStrategy, MessageDeduplicator};
pub use exchange_config::ExchangeConfig;
pub use factory::ExchangeFactory;
pub use filter::{MarketFilter, MarketFilterConfig};
pub use pool::{ConnectionPool, StreamFactory};
pub use reconnecting::ReconnectingDataStream;
pub use scorer::MarketScorer;
