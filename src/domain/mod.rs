//! Exchange-agnostic domain logic.

mod detector;
mod ids;
mod market;
mod money;
mod orderbook;
mod types;

// New properly-encapsulated types from focused modules
pub use ids::{MarketId, TokenId};
pub use market::{MarketInfo, MarketPair, TokenInfo};
pub use money::{Price, Volume};

// Detector and cache
pub use detector::{detect_single_condition, DetectorConfig};
pub use orderbook::OrderBookCache;

// Types from types.rs (kept for backward compatibility, will be removed in Task 14)
// Note: Opportunity, OrderBook, PriceLevel still live in types.rs
pub use types::{Opportunity, OrderBook, PriceLevel};
