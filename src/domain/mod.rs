//! Pure domain types. No I/O, no external dependencies.

mod execution;
mod id;
mod market;
mod market_registry;
mod money;
mod monitoring;
mod opportunity;
mod order_book;
mod position;
mod relation;
mod resource;
mod scaling;
mod score;

pub use execution::{ArbitrageExecutionResult, FailedLeg, FilledLeg, OrderId};
pub use id::{ClusterId, MarketId, RelationId, TokenId};
pub use market::{Market, Outcome};
pub use market_registry::MarketRegistry;
pub use money::{Price, Volume};
pub use monitoring::PoolStats;
pub use opportunity::{Opportunity, OpportunityLeg};
pub use order_book::{OrderBook, PriceLevel};
pub use position::{Position, PositionId, PositionLeg, PositionStatus};
pub use relation::{Cluster, Relation, RelationKind};
pub use resource::ResourceBudget;
pub use scaling::ScalingRecommendation;
pub use score::{MarketScore, ScoreFactors, ScoreWeights};
