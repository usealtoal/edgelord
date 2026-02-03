//! Exchange configuration traits.
//!
//! This module defines traits for exchange-specific configuration,
//! allowing different exchanges to provide their own payout amounts
//! and outcome naming conventions.

use rust_decimal::Decimal;

/// Configuration trait for exchange-specific settings.
///
/// Different exchanges have different payout amounts and outcome naming conventions.
/// Implementations of this trait provide these exchange-specific configurations.
///
/// # Examples
///
/// ```
/// use rust_decimal_macros::dec;
/// use edgelord::core::exchange::ExchangeConfig;
///
/// struct MyExchange;
///
/// impl ExchangeConfig for MyExchange {
///     fn name(&self) -> &'static str {
///         "my-exchange"
///     }
///
///     fn default_payout(&self) -> rust_decimal::Decimal {
///         dec!(1.00)
///     }
///
///     fn binary_outcome_names(&self) -> (&'static str, &'static str) {
///         ("Yes", "No")
///     }
/// }
/// ```
pub trait ExchangeConfig: Send + Sync {
    /// Returns the exchange name for logging and identification.
    fn name(&self) -> &'static str;

    /// Returns the default payout amount for winning outcomes.
    ///
    /// For example, Polymarket uses $1.00 per share.
    fn default_payout(&self) -> Decimal;

    /// Returns the binary outcome names as (positive, negative).
    ///
    /// For example, Polymarket uses ("Yes", "No").
    fn binary_outcome_names(&self) -> (&'static str, &'static str);

    /// Checks if the given name represents a positive outcome (case-insensitive).
    ///
    /// # Arguments
    ///
    /// * `name` - The outcome name to check
    ///
    /// # Returns
    ///
    /// `true` if the name matches the positive outcome name (case-insensitive)
    fn is_positive_outcome(&self, name: &str) -> bool {
        let (positive, _) = self.binary_outcome_names();
        name.eq_ignore_ascii_case(positive)
    }

    /// Checks if the given name represents a negative outcome (case-insensitive).
    ///
    /// # Arguments
    ///
    /// * `name` - The outcome name to check
    ///
    /// # Returns
    ///
    /// `true` if the name matches the negative outcome name (case-insensitive)
    fn is_negative_outcome(&self, name: &str) -> bool {
        let (_, negative) = self.binary_outcome_names();
        name.eq_ignore_ascii_case(negative)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Mock exchange implementation for testing.
    struct MockExchange;

    impl ExchangeConfig for MockExchange {
        fn name(&self) -> &'static str {
            "mock-exchange"
        }

        fn default_payout(&self) -> Decimal {
            dec!(1.00)
        }

        fn binary_outcome_names(&self) -> (&'static str, &'static str) {
            ("Yes", "No")
        }
    }

    /// Custom exchange with different outcome names.
    struct CustomExchange;

    impl ExchangeConfig for CustomExchange {
        fn name(&self) -> &'static str {
            "custom-exchange"
        }

        fn default_payout(&self) -> Decimal {
            dec!(100.00)
        }

        fn binary_outcome_names(&self) -> (&'static str, &'static str) {
            ("True", "False")
        }
    }

    #[test]
    fn test_name_returns_exchange_name() {
        let exchange = MockExchange;
        assert_eq!(exchange.name(), "mock-exchange");
    }

    #[test]
    fn test_default_payout_returns_configured_value() {
        let exchange = MockExchange;
        assert_eq!(exchange.default_payout(), dec!(1.00));

        let custom = CustomExchange;
        assert_eq!(custom.default_payout(), dec!(100.00));
    }

    #[test]
    fn test_binary_outcome_names_returns_positive_negative_tuple() {
        let exchange = MockExchange;
        let (positive, negative) = exchange.binary_outcome_names();
        assert_eq!(positive, "Yes");
        assert_eq!(negative, "No");
    }

    #[test]
    fn test_is_positive_outcome_exact_match() {
        let exchange = MockExchange;
        assert!(exchange.is_positive_outcome("Yes"));
    }

    #[test]
    fn test_is_positive_outcome_case_insensitive() {
        let exchange = MockExchange;
        assert!(exchange.is_positive_outcome("yes"));
        assert!(exchange.is_positive_outcome("YES"));
        assert!(exchange.is_positive_outcome("yEs"));
    }

    #[test]
    fn test_is_positive_outcome_returns_false_for_negative() {
        let exchange = MockExchange;
        assert!(!exchange.is_positive_outcome("No"));
        assert!(!exchange.is_positive_outcome("no"));
    }

    #[test]
    fn test_is_positive_outcome_returns_false_for_unrelated() {
        let exchange = MockExchange;
        assert!(!exchange.is_positive_outcome("Maybe"));
        assert!(!exchange.is_positive_outcome(""));
        assert!(!exchange.is_positive_outcome("Yess")); // Not exact match
    }

    #[test]
    fn test_is_negative_outcome_exact_match() {
        let exchange = MockExchange;
        assert!(exchange.is_negative_outcome("No"));
    }

    #[test]
    fn test_is_negative_outcome_case_insensitive() {
        let exchange = MockExchange;
        assert!(exchange.is_negative_outcome("no"));
        assert!(exchange.is_negative_outcome("NO"));
        assert!(exchange.is_negative_outcome("nO"));
    }

    #[test]
    fn test_is_negative_outcome_returns_false_for_positive() {
        let exchange = MockExchange;
        assert!(!exchange.is_negative_outcome("Yes"));
        assert!(!exchange.is_negative_outcome("yes"));
    }

    #[test]
    fn test_is_negative_outcome_returns_false_for_unrelated() {
        let exchange = MockExchange;
        assert!(!exchange.is_negative_outcome("Maybe"));
        assert!(!exchange.is_negative_outcome(""));
        assert!(!exchange.is_negative_outcome("Nope")); // Not exact match
    }

    #[test]
    fn test_custom_exchange_outcome_names() {
        let exchange = CustomExchange;
        assert!(exchange.is_positive_outcome("True"));
        assert!(exchange.is_positive_outcome("true"));
        assert!(exchange.is_negative_outcome("False"));
        assert!(exchange.is_negative_outcome("FALSE"));
    }

    #[test]
    fn test_trait_is_object_safe() {
        // Verify the trait can be used as a trait object
        let exchange: &dyn ExchangeConfig = &MockExchange;
        assert_eq!(exchange.name(), "mock-exchange");
        assert!(exchange.is_positive_outcome("Yes"));
    }

    #[test]
    fn test_trait_is_send_sync() {
        // Verify the trait bounds at compile time
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MockExchange>();

        // Also verify trait object is Send + Sync
        fn assert_trait_object_send_sync<T: Send + Sync + ?Sized>() {}
        assert_trait_object_send_sync::<dyn ExchangeConfig>();
    }
}
