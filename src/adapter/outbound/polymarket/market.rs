//! Polymarket-specific market parser.
//!
//! Maps Polymarket market payloads into domain markets.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::port::outbound::exchange::MarketParser;

/// Default payout amount for Polymarket ($1.00 per share).
pub const POLYMARKET_PAYOUT: Decimal = dec!(1);

/// Polymarket market parser.
///
/// Polymarket uses:
/// - $1.00 payout per winning share
/// - "Yes"/"No" outcome naming convention
///
/// # Examples
///
/// ```
/// use edgelord::adapter::outbound::polymarket::market::PolymarketMarketParser;
/// use edgelord::port::outbound::exchange::MarketParser;
/// use rust_decimal_macros::dec;
///
/// let parser = PolymarketMarketParser;
/// assert_eq!(parser.name(), "polymarket");
/// assert_eq!(parser.default_payout(), dec!(1.00));
/// assert_eq!(parser.binary_outcome_names(), ("Yes", "No"));
/// ```
pub struct PolymarketMarketParser;

impl MarketParser for PolymarketMarketParser {
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
        let parser = PolymarketMarketParser;
        assert_eq!(parser.name(), "polymarket");
    }

    #[test]
    fn test_default_payout_returns_one_dollar() {
        let parser = PolymarketMarketParser;
        assert_eq!(parser.default_payout(), dec!(1.00));
        assert_eq!(parser.default_payout(), Decimal::ONE);
    }

    #[test]
    fn test_binary_outcome_names_returns_yes_no() {
        let parser = PolymarketMarketParser;
        let (positive, negative) = parser.binary_outcome_names();
        assert_eq!(positive, "Yes");
        assert_eq!(negative, "No");
    }

    #[test]
    fn test_is_positive_outcome_for_yes_variants() {
        let parser = PolymarketMarketParser;
        assert!(parser.is_positive_outcome("Yes"));
        assert!(parser.is_positive_outcome("yes"));
        assert!(parser.is_positive_outcome("YES"));
        assert!(parser.is_positive_outcome("yEs"));
    }

    #[test]
    fn test_is_positive_outcome_returns_false_for_no() {
        let parser = PolymarketMarketParser;
        assert!(!parser.is_positive_outcome("No"));
        assert!(!parser.is_positive_outcome("no"));
    }

    #[test]
    fn test_is_negative_outcome_for_no_variants() {
        let parser = PolymarketMarketParser;
        assert!(parser.is_negative_outcome("No"));
        assert!(parser.is_negative_outcome("no"));
        assert!(parser.is_negative_outcome("NO"));
        assert!(parser.is_negative_outcome("nO"));
    }

    #[test]
    fn test_is_negative_outcome_returns_false_for_yes() {
        let parser = PolymarketMarketParser;
        assert!(!parser.is_negative_outcome("Yes"));
        assert!(!parser.is_negative_outcome("yes"));
    }

    #[test]
    fn test_config_is_unit_struct() {
        // Verify it's a unit struct (no fields)
        let _parser = PolymarketMarketParser;
        // If this compiles, it's a unit struct
    }

    #[test]
    fn test_config_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PolymarketMarketParser>();
    }

    #[test]
    fn test_config_as_trait_object() {
        let parser: &dyn MarketParser = &PolymarketMarketParser;
        assert_eq!(parser.name(), "polymarket");
        assert_eq!(parser.default_payout(), dec!(1.00));
    }
}
