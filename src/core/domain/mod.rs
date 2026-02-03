//! Pure domain types.

mod id;
mod market;
mod market_registry;
mod money;
mod opportunity;
mod orderbook;
mod position;

pub use id::{MarketId, TokenId};
pub use market::{Market, MarketPair, Outcome};
pub use market_registry::MarketRegistry;
pub use money::{Price, Volume};
pub use opportunity::{Opportunity, OpportunityLeg};
pub use orderbook::{OrderBook, PriceLevel};
pub use position::{Position, PositionId, PositionLeg, PositionStatus};
