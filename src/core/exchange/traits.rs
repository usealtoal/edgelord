//! Exchange configuration traits.
//!
//! This module defines traits for exchange-specific configuration,
//! allowing different exchanges to provide their own payout amounts
//! and outcome naming conventions.

use rust_decimal::Decimal;

use super::MarketInfo;
use crate::core::domain::{Market, MarketId, Outcome, TokenId};

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

    /// Parse exchange-agnostic market info into generic `Market` objects.
    ///
    /// The default implementation filters to binary markets with outcomes
    /// matching the exchange's `binary_outcome_names()` and uses the
    /// exchange's `default_payout()`.
    ///
    /// Exchanges with different parsing requirements can override this method.
    ///
    /// # Arguments
    ///
    /// * `market_infos` - Exchange-agnostic market information
    ///
    /// # Returns
    ///
    /// A vector of generic `Market` objects ready for strategy detection
    fn parse_markets(&self, market_infos: &[MarketInfo]) -> Vec<Market> {
        let mut markets = Vec::new();
        let (positive_name, negative_name) = self.binary_outcome_names();
        let payout = self.default_payout();

        for info in market_infos {
            // Only process binary markets
            if info.outcomes.len() != 2 {
                continue;
            }

            let positive = info
                .outcomes
                .iter()
                .find(|o| self.is_positive_outcome(&o.name));
            let negative = info
                .outcomes
                .iter()
                .find(|o| self.is_negative_outcome(&o.name));

            if let (Some(pos), Some(neg)) = (positive, negative) {
                let outcomes = vec![
                    Outcome::new(TokenId::from(pos.token_id.clone()), positive_name),
                    Outcome::new(TokenId::from(neg.token_id.clone()), negative_name),
                ];
                let market = Market::new(
                    MarketId::from(info.id.clone()),
                    info.question.clone(),
                    outcomes,
                    payout,
                );
                markets.push(market);
            }
        }

        markets
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

    // --- parse_markets tests ---

    fn make_market_info(id: &str, question: &str, outcomes: Vec<(&str, &str)>) -> MarketInfo {
        use crate::core::exchange::OutcomeInfo;
        MarketInfo {
            id: id.to_string(),
            question: question.to_string(),
            outcomes: outcomes
                .into_iter()
                .map(|(token_id, name)| OutcomeInfo {
                    token_id: token_id.to_string(),
                    name: name.to_string(),
                    price: None,
                })
                .collect(),
            active: true,
        }
    }

    #[test]
    fn test_parse_markets_converts_binary_yes_no() {
        let exchange = MockExchange;
        let infos = vec![
            make_market_info("m1", "Question 1?", vec![("yes-1", "Yes"), ("no-1", "No")]),
            make_market_info("m2", "Question 2?", vec![("yes-2", "Yes"), ("no-2", "No")]),
        ];

        let markets = exchange.parse_markets(&infos);

        assert_eq!(markets.len(), 2);
        assert_eq!(markets[0].market_id().as_str(), "m1");
        assert_eq!(markets[0].question(), "Question 1?");
        assert_eq!(markets[0].payout(), dec!(1.00));
        assert_eq!(markets[0].outcome_count(), 2);
    }

    #[test]
    fn test_parse_markets_uses_exchange_payout() {
        let exchange = CustomExchange;
        let infos = vec![make_market_info(
            "m1",
            "Q?",
            vec![("t", "True"), ("f", "False")],
        )];

        let markets = exchange.parse_markets(&infos);

        assert_eq!(markets.len(), 1);
        assert_eq!(markets[0].payout(), dec!(100.00));
    }

    #[test]
    fn test_parse_markets_skips_non_binary() {
        let exchange = MockExchange;
        let infos = vec![make_market_info(
            "m1",
            "Multi?",
            vec![("a", "Yes"), ("b", "No"), ("c", "Maybe")],
        )];

        let markets = exchange.parse_markets(&infos);

        assert!(markets.is_empty());
    }

    #[test]
    fn test_parse_markets_skips_non_matching_outcomes() {
        let exchange = MockExchange;
        let infos = vec![make_market_info(
            "m1",
            "Colors?",
            vec![("r", "Red"), ("b", "Blue")],
        )];

        let markets = exchange.parse_markets(&infos);

        assert!(markets.is_empty());
    }

    #[test]
    fn test_parse_markets_case_insensitive() {
        let exchange = MockExchange;
        let infos = vec![make_market_info(
            "m1",
            "Q?",
            vec![("y", "YES"), ("n", "no")],
        )];

        let markets = exchange.parse_markets(&infos);

        assert_eq!(markets.len(), 1);
    }

    #[test]
    fn test_parse_markets_empty_input() {
        let exchange = MockExchange;
        let markets = exchange.parse_markets(&[]);
        assert!(markets.is_empty());
    }

    #[test]
    fn test_parse_markets_custom_exchange_outcome_names() {
        let exchange = CustomExchange;
        let infos = vec![
            // Should parse - matches True/False
            make_market_info("m1", "Q1?", vec![("t1", "True"), ("f1", "False")]),
            // Should NOT parse - Yes/No doesn't match True/False exchange
            make_market_info("m2", "Q2?", vec![("y", "Yes"), ("n", "No")]),
        ];

        let markets = exchange.parse_markets(&infos);

        assert_eq!(markets.len(), 1);
        assert_eq!(markets[0].market_id().as_str(), "m1");
    }
}
