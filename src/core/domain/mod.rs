//! Core domain types for edgelord.
//!
//! ## Market Types
//!
//! - [`Market`] - Generic market supporting N outcomes with configurable payout
//! - [`Outcome`] - A single outcome within a market
//! - [`MarketRegistry`] - Registry mapping token IDs to markets
//!
//! ## Opportunity Types
//!
//! - [`Opportunity`] - Detected arbitrage opportunity with N legs
//! - [`OpportunityLeg`] - A single leg (token purchase) in an opportunity
//!
//! ## Scoring Types
//!
//! - [`ScoreFactors`] - Individual scoring factors for a market
//! - [`ScoreWeights`] - Weights for combining factors into a composite score
//! - [`MarketScore`] - A market's computed score for subscription prioritization
//!
//! ## Resource Types
//!
//! - [`ResourceBudget`] - Resource constraints for adaptive subscription scaling
//!
//! ## Scaling Types
//!
//! - [`ScalingRecommendation`] - Scaling decision from the AdaptiveGovernor
//!
//! ## Identifier Types
//!
//! - [`MarketId`] - Unique market identifier
//! - [`TokenId`] - Unique token/outcome identifier

mod id;
mod market;
mod market_registry;
mod money;
mod opportunity;
mod order_book;
mod position;
mod resource;
mod scaling;
mod score;

pub use id::{MarketId, TokenId};
pub use market::{Market, Outcome};
pub use market_registry::MarketRegistry;
pub use money::{Price, Volume};
pub use opportunity::{Opportunity, OpportunityLeg};
pub use order_book::{OrderBook, PriceLevel};
pub use position::{Position, PositionId, PositionLeg, PositionStatus};
pub use resource::ResourceBudget;
pub use scaling::ScalingRecommendation;
pub use score::{MarketScore, ScoreFactors, ScoreWeights};
