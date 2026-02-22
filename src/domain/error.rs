//! Domain validation errors for core domain types.

use thiserror::Error;

/// Domain validation errors for domain constructors and invariants.
#[derive(Error, Debug, Clone)]
pub enum DomainError {
    #[error("volume must be positive, got {volume}")]
    NonPositiveVolume { volume: rust_decimal::Decimal },

    #[error("payout {payout} must be greater than cost {cost}")]
    PayoutNotGreaterThanCost {
        payout: rust_decimal::Decimal,
        cost: rust_decimal::Decimal,
    },

    #[error("outcomes cannot be empty")]
    EmptyOutcomes,

    #[error("payout must be positive, got {payout}")]
    NonPositivePayout { payout: rust_decimal::Decimal },

    #[error("legs cannot be empty")]
    EmptyLegs,
}
