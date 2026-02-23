//! Pure domain types for prediction market arbitrage.
//!
//! This module contains the core domain model with no I/O or external dependencies.
//! All types are designed to be serializable, testable, and independent of any
//! specific exchange or infrastructure implementation.
//!
//! # Module Organization
//!
//! - [`id`] - Type-safe identifiers for markets, tokens, orders, and positions
//! - [`market`] - Market and outcome representations
//! - [`book`] - Order book price levels and depth
//! - [`opportunity`] - Detected arbitrage opportunities
//! - [`position`] - Open and closed trading positions
//! - [`trade`] - Trade execution results and fill information
//! - [`relation`] - Logical relations between markets (implies, exclusive, etc.)
//! - [`cluster`] - Groups of related markets with pre-computed constraints
//! - [`constraint`] - Linear constraints for optimization problems
//! - [`score`] - Market scoring for subscription prioritization
//! - [`money`] - Price and volume type aliases
//! - [`stats`] - Trading statistics and summaries
//! - [`error`] - Domain validation errors
//!
//! # Examples
//!
//! Creating a binary market:
//!
//! ```ignore
//! use edgelord::domain::{market::{Market, Outcome}, id::{MarketId, TokenId}};
//! use rust_decimal_macros::dec;
//!
//! let market = Market::new(
//!     MarketId::new("election-2024"),
//!     "Will candidate X win?",
//!     vec![
//!         Outcome::new(TokenId::new("yes-token"), "Yes"),
//!         Outcome::new(TokenId::new("no-token"), "No"),
//!     ],
//!     dec!(1.00),
//! );
//! ```
//!
//! Detecting an arbitrage opportunity:
//!
//! ```ignore
//! use edgelord::domain::{opportunity::{Opportunity, OpportunityLeg}, id::{MarketId, TokenId}};
//! use rust_decimal_macros::dec;
//!
//! let legs = vec![
//!     OpportunityLeg::new(TokenId::new("yes"), dec!(0.45)),
//!     OpportunityLeg::new(TokenId::new("no"), dec!(0.50)),
//! ];
//! let opp = Opportunity::new(
//!     MarketId::new("market-1"),
//!     "Will it rain?",
//!     legs,
//!     dec!(100), // volume
//!     dec!(1.00), // payout
//! );
//! assert_eq!(opp.edge(), dec!(0.05)); // 5 cent edge per share
//! ```

pub mod book;
pub mod cluster;
pub mod constraint;
pub mod error;
pub mod id;
pub mod market;
pub mod money;
pub mod opportunity;
pub mod position;
pub mod relation;
pub mod score;
pub mod stats;
pub mod trade;
