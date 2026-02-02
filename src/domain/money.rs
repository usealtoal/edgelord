//! Monetary types for price and volume representation.

use rust_decimal::Decimal;

/// Price represented as a Decimal for precision.
pub type Price = Decimal;

/// Volume represented as a Decimal for precision.
pub type Volume = Decimal;

/// Common monetary constants.
pub mod constants {
    use rust_decimal::Decimal;

    /// One dollar.
    pub const ONE_DOLLAR: Decimal = Decimal::ONE;

    /// Zero dollars.
    pub const ZERO: Decimal = Decimal::ZERO;
}

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

    #[test]
    fn constants_are_correct() {
        assert_eq!(constants::ONE_DOLLAR, Decimal::ONE);
        assert_eq!(constants::ZERO, Decimal::ZERO);
    }
}
