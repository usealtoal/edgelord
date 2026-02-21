//! Polymarket-specific exchange configuration.
//!
//! This module provides the `PolymarketExchangeConfig` implementation
//! of the `ExchangeConfig` trait for Polymarket-specific settings.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::core::exchange::ExchangeConfig;

/// Default payout amount for Polymarket ($1.00 per share).
pub const POLYMARKET_PAYOUT: Decimal = dec!(1);

/// Polymarket-specific exchange configuration.
///
/// Polymarket uses:
/// - $1.00 payout per winning share
/// - "Yes"/"No" outcome naming convention
///
/// # Examples
///
/// ```
/// use edgelord::core::exchange::polymarket::PolymarketExchangeConfig;
/// use edgelord::core::exchange::ExchangeConfig;
/// use rust_decimal_macros::dec;
///
/// let config = PolymarketExchangeConfig;
/// assert_eq!(config.name(), "polymarket");
/// assert_eq!(config.default_payout(), dec!(1.00));
/// assert_eq!(config.binary_outcome_names(), ("Yes", "No"));
/// ```
pub struct PolymarketExchangeConfig;

impl ExchangeConfig for PolymarketExchangeConfig {
    fn name(&self) -> &'static str {
        "polymarket"
    }

    fn default_payout(&self) -> Decimal {
        POLYMARKET_PAYOUT
    }

    fn binary_outcome_names(&self) -> (&'static str, &'static str) {
        ("Yes", "No")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_polymarket_payout_constant_is_one_dollar() {
        assert_eq!(POLYMARKET_PAYOUT, dec!(1));
        assert_eq!(POLYMARKET_PAYOUT, Decimal::ONE);
    }

    #[test]
    fn test_name_returns_polymarket() {
        let config = PolymarketExchangeConfig;
        assert_eq!(config.name(), "polymarket");
    }

    #[test]
    fn test_default_payout_returns_one_dollar() {
        let config = PolymarketExchangeConfig;
        assert_eq!(config.default_payout(), dec!(1.00));
        assert_eq!(config.default_payout(), Decimal::ONE);
    }

    #[test]
    fn test_binary_outcome_names_returns_yes_no() {
        let config = PolymarketExchangeConfig;
        let (positive, negative) = config.binary_outcome_names();
        assert_eq!(positive, "Yes");
        assert_eq!(negative, "No");
    }

    #[test]
    fn test_is_positive_outcome_for_yes_variants() {
        let config = PolymarketExchangeConfig;
        assert!(config.is_positive_outcome("Yes"));
        assert!(config.is_positive_outcome("yes"));
        assert!(config.is_positive_outcome("YES"));
        assert!(config.is_positive_outcome("yEs"));
    }

    #[test]
    fn test_is_positive_outcome_returns_false_for_no() {
        let config = PolymarketExchangeConfig;
        assert!(!config.is_positive_outcome("No"));
        assert!(!config.is_positive_outcome("no"));
    }

    #[test]
    fn test_is_negative_outcome_for_no_variants() {
        let config = PolymarketExchangeConfig;
        assert!(config.is_negative_outcome("No"));
        assert!(config.is_negative_outcome("no"));
        assert!(config.is_negative_outcome("NO"));
        assert!(config.is_negative_outcome("nO"));
    }

    #[test]
    fn test_is_negative_outcome_returns_false_for_yes() {
        let config = PolymarketExchangeConfig;
        assert!(!config.is_negative_outcome("Yes"));
        assert!(!config.is_negative_outcome("yes"));
    }

    #[test]
    fn test_config_is_unit_struct() {
        // Verify it's a unit struct (no fields)
        let _config = PolymarketExchangeConfig;
        // If this compiles, it's a unit struct
    }

    #[test]
    fn test_config_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PolymarketExchangeConfig>();
    }

    #[test]
    fn test_config_as_trait_object() {
        let config: &dyn ExchangeConfig = &PolymarketExchangeConfig;
        assert_eq!(config.name(), "polymarket");
        assert_eq!(config.default_payout(), dec!(1.00));
    }
}
