//! Domain validation errors for core domain types.
//!
//! This module defines errors that occur when domain invariants are violated.
//! These errors are returned by `try_new` constructors that validate inputs.
//!
//! # Examples
//!
//! Handling validation errors:
//!
//! ```
//! use edgelord::domain::error::DomainError;
//! use edgelord::domain::market::{Market, Outcome};
//! use edgelord::domain::id::{MarketId, TokenId};
//! use rust_decimal_macros::dec;
//!
//! // Empty outcomes will fail validation
//! let result = Market::try_new(
//!     MarketId::new("market-1"),
//!     "Test market?",
//!     vec![],  // empty!
//!     dec!(1.00),
//! );
//!
//! assert!(matches!(result, Err(DomainError::EmptyOutcomes)));
//! ```

use thiserror::Error;

/// Errors that occur when domain invariants are violated.
///
/// These errors are returned by `try_new` constructors and other methods
/// that validate domain rules.
#[derive(Error, Debug, Clone)]
pub enum DomainError {
    /// Volume must be positive for trading operations.
    #[error("volume must be positive, got {volume}")]
    NonPositiveVolume {
        /// The invalid volume that was provided.
        volume: rust_decimal::Decimal,
    },

    /// Payout must exceed cost for a valid arbitrage opportunity.
    #[error("payout {payout} must be greater than cost {cost}")]
    PayoutNotGreaterThanCost {
        /// The payout amount.
        payout: rust_decimal::Decimal,
        /// The cost amount.
        cost: rust_decimal::Decimal,
    },

    /// Markets must have at least one outcome.
    #[error("outcomes cannot be empty")]
    EmptyOutcomes,

    /// Payout must be positive for valid market resolution.
    #[error("payout must be positive, got {payout}")]
    NonPositivePayout {
        /// The invalid payout that was provided.
        payout: rust_decimal::Decimal,
    },

    /// Positions must have at least one leg.
    #[error("legs cannot be empty")]
    EmptyLegs,
}
