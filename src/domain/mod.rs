//! Pure domain types. No I/O, no external dependencies.

mod book;
mod cluster;
mod id;
mod market;
mod money;
mod opportunity;
mod position;
mod relation;
mod trade;

pub use book::{Book, PriceLevel};
pub use cluster::Cluster;
pub use id::{ClusterId, MarketId, OrderId, PositionId, RelationId, TokenId};
pub use market::{Market, MarketRegistry, Outcome};
pub use money::{Price, Volume};
pub use opportunity::{Opportunity, OpportunityLeg};
pub use position::{Position, PositionLeg, PositionStatus};
pub use relation::{Relation, RelationKind};
pub use trade::{Failure, Fill, TradeResult};
