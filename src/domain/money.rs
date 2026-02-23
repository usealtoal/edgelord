//! Monetary types for price and volume representation.
//!
//! This module provides type aliases for monetary values using [`Decimal`]
//! for precise arithmetic. Using fixed-point decimal arithmetic avoids
//! floating-point rounding errors in financial calculations.
//!
//! # Type Aliases
//!
//! - [`Price`] - A price value (e.g., $0.45 per share)
//! - [`Volume`] - A volume/quantity value (e.g., 100 shares)
//!
//! # Examples
//!
//! ```
//! use edgelord::domain::money::{Price, Volume};
//! use rust_decimal_macros::dec;
//!
//! let price: Price = dec!(0.45);
//! let volume: Volume = dec!(100);
//! let total: Price = price * volume;
//!
//! assert_eq!(total, dec!(45.00));
//! ```

use rust_decimal::Decimal;

/// Price value represented as a decimal for precision.
///
/// Uses [`Decimal`] to avoid floating-point rounding errors in
/// financial calculations.
pub type Price = Decimal;

/// Volume (quantity) value represented as a decimal for precision.
///
/// Uses [`Decimal`] to support fractional share quantities while
/// maintaining arithmetic precision.
pub type Volume = Decimal;

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn price_and_volume_are_decimal() {
        let price: Price = dec!(1.50);
        let volume: Volume = dec!(100.0);

        assert_eq!(price + volume, dec!(101.50));
    }
}
