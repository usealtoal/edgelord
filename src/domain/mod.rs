//! Exchange-agnostic domain logic.

mod detector;
mod orderbook;
mod types;

pub use detector::{detect_single_condition, DetectorConfig};
pub use orderbook::OrderBookCache;
pub use types::{MarketId, MarketPair, Opportunity, OrderBook, PriceLevel, TokenId};
