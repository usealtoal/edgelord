//! Exchange-agnostic domain logic.

mod id;
mod market;
mod money;
mod opportunity;
mod orderbook;
mod position;

pub mod solver;
pub mod strategy;

// Core domain types
pub use id::{MarketId, TokenId};
pub use market::{DomainMarket, MarketPair, TokenInfo};
pub use money::{Price, Volume};
pub use opportunity::{Opportunity, OpportunityBuildError, OpportunityBuilder};
pub use position::{Position, PositionId, PositionLeg, PositionStatus, PositionTracker};

// OrderBook types and cache
pub use orderbook::{OrderBook, OrderBookCache, PriceLevel};

// Detector (legacy re-export for backwards compatibility)
pub use strategy::single_condition::{detect_single_condition, SingleConditionConfig as DetectorConfig};
