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
//! ## Identifier Types
//!
//! - [`MarketId`] - Unique market identifier
//! - [`TokenId`] - Unique token/outcome identifier

mod id;
mod market;
mod market_registry;
mod money;
mod opportunity;
mod orderbook;
mod position;

pub use id::{MarketId, TokenId};
pub use market::{Market, Outcome};
pub use market_registry::MarketRegistry;
pub use money::{Price, Volume};
pub use opportunity::{Opportunity, OpportunityLeg};
pub use orderbook::{OrderBook, PriceLevel};
pub use position::{Position, PositionId, PositionLeg, PositionStatus};
