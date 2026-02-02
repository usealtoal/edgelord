//! Exchange-agnostic domain logic.

mod detector;
mod ids;
mod market;
mod money;
mod opportunity;
mod orderbook;
mod position;

pub mod solver;
pub mod strategy;

// Core domain types
pub use ids::{MarketId, TokenId};
pub use market::{MarketInfo, MarketPair, TokenInfo};
pub use money::{Price, Volume};
pub use opportunity::{Opportunity, OpportunityBuildError, OpportunityBuilder};
pub use position::{Position, PositionId, PositionLeg, PositionStatus, PositionTracker};

// OrderBook types and cache
pub use orderbook::{OrderBook, OrderBookCache, PriceLevel};

// Detector
pub use detector::{detect_single_condition, DetectorConfig};
