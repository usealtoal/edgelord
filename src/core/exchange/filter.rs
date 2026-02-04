//! Market filtering trait for subscription eligibility.
//!
//! Exchanges implement [`MarketFilter`] to determine which markets are eligible
//! for subscription based on configurable criteria like volume, liquidity, and spread.

use super::MarketInfo;

/// Configuration for market filtering.
///
/// Defines the criteria used to filter markets for subscription eligibility.
/// Markets must meet all configured thresholds to be considered eligible.
///
/// # Example
///
/// ```
/// use edgelord::core::exchange::MarketFilterConfig;
///
/// // Use defaults
/// let config = MarketFilterConfig::default();
///
/// // Or customize
/// let config = MarketFilterConfig {
///     max_markets: 500,
///     min_volume_24h: 10_000.0,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct MarketFilterConfig {
    /// Maximum number of markets to consider for subscription.
    pub max_markets: usize,
    /// Maximum number of active subscriptions allowed.
    pub max_subscriptions: usize,
    /// Minimum 24-hour trading volume (in USD).
    pub min_volume_24h: f64,
    /// Minimum liquidity depth (in USD).
    pub min_liquidity: f64,
    /// Maximum bid-ask spread as a percentage (e.g., 5.0 = 5%).
    pub max_spread_pct: f64,
    /// Whether to include binary (YES/NO) markets.
    pub include_binary: bool,
    /// Whether to include multi-outcome markets.
    pub include_multi_outcome: bool,
    /// Maximum number of outcomes allowed in a market.
    pub max_outcomes: usize,
}

impl Default for MarketFilterConfig {
    fn default() -> Self {
        Self {
            max_markets: 1000,
            max_subscriptions: 100,
            min_volume_24h: 1000.0,
            min_liquidity: 500.0,
            max_spread_pct: 10.0,
            include_binary: true,
            include_multi_outcome: true,
            max_outcomes: 10,
        }
    }
}

/// Filters markets for subscription eligibility.
///
/// Implementations analyze market characteristics and determine which markets
/// meet the criteria for active subscription. This is typically the first step
/// in the subscription prioritization pipeline, filtering out markets that don't
/// meet minimum thresholds before scoring.
///
/// # Example
///
/// ```ignore
/// struct MyExchangeFilter {
///     config: MarketFilterConfig,
/// }
///
/// impl MarketFilter for MyExchangeFilter {
///     fn is_eligible(&self, market: &MarketInfo) -> bool {
///         // Check if market meets criteria
///         market.active
///             && market.outcomes.len() <= self.config.max_outcomes
///             && (self.config.include_binary || market.outcomes.len() != 2)
///     }
///
///     fn config(&self) -> &MarketFilterConfig {
///         &self.config
///     }
///
///     fn exchange_name(&self) -> &'static str {
///         "myexchange"
///     }
/// }
/// ```
pub trait MarketFilter: Send + Sync {
    /// Check if a single market is eligible for subscription.
    ///
    /// Returns `true` if the market meets all filtering criteria.
    ///
    /// # Arguments
    ///
    /// * `market` - Market information to evaluate
    fn is_eligible(&self, market: &MarketInfo) -> bool;

    /// Filter a slice of markets, returning only eligible ones.
    ///
    /// Default implementation calls [`is_eligible`](Self::is_eligible) for each market.
    /// Implementations may override this for more efficient batch processing.
    ///
    /// # Arguments
    ///
    /// * `markets` - Slice of markets to filter
    fn filter(&self, markets: &[MarketInfo]) -> Vec<MarketInfo> {
        markets
            .iter()
            .filter(|m| self.is_eligible(m))
            .cloned()
            .collect()
    }

    /// Get the filter configuration.
    fn config(&self) -> &MarketFilterConfig;

    /// Get the exchange name for logging and debugging.
    fn exchange_name(&self) -> &'static str;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::exchange::OutcomeInfo;

    // --- MarketFilterConfig tests ---

    #[test]
    fn config_default_has_sensible_values() {
        let config = MarketFilterConfig::default();

        assert_eq!(config.max_markets, 1000);
        assert_eq!(config.max_subscriptions, 100);
        assert!((config.min_volume_24h - 1000.0).abs() < f64::EPSILON);
        assert!((config.min_liquidity - 500.0).abs() < f64::EPSILON);
        assert!((config.max_spread_pct - 10.0).abs() < f64::EPSILON);
        assert!(config.include_binary);
        assert!(config.include_multi_outcome);
        assert_eq!(config.max_outcomes, 10);
    }

    #[test]
    fn config_can_be_customized() {
        let config = MarketFilterConfig {
            max_markets: 500,
            max_subscriptions: 50,
            min_volume_24h: 5000.0,
            min_liquidity: 2000.0,
            max_spread_pct: 5.0,
            include_binary: false,
            include_multi_outcome: true,
            max_outcomes: 5,
        };

        assert_eq!(config.max_markets, 500);
        assert_eq!(config.max_subscriptions, 50);
        assert!((config.min_volume_24h - 5000.0).abs() < f64::EPSILON);
        assert!(!config.include_binary);
    }

    #[test]
    fn config_partial_customization_with_defaults() {
        let config = MarketFilterConfig {
            max_markets: 200,
            ..Default::default()
        };

        assert_eq!(config.max_markets, 200);
        assert_eq!(config.max_subscriptions, 100); // From default
        assert!(config.include_binary); // From default
    }

    #[test]
    fn config_implements_clone() {
        let config = MarketFilterConfig::default();
        let cloned = config.clone();
        assert_eq!(config, cloned);
    }

    #[test]
    fn config_implements_debug() {
        let config = MarketFilterConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("MarketFilterConfig"));
        assert!(debug_str.contains("max_markets"));
    }

    // --- MarketFilter trait tests ---

    /// Mock filter implementation for testing.
    struct MockFilter {
        config: MarketFilterConfig,
        only_active: bool,
    }

    impl MockFilter {
        fn new(config: MarketFilterConfig) -> Self {
            Self {
                config,
                only_active: true,
            }
        }

        fn accepting_all(config: MarketFilterConfig) -> Self {
            Self {
                config,
                only_active: false,
            }
        }
    }

    impl MarketFilter for MockFilter {
        fn is_eligible(&self, market: &MarketInfo) -> bool {
            if self.only_active {
                market.active && market.outcomes.len() <= self.config.max_outcomes
            } else {
                true
            }
        }

        fn config(&self) -> &MarketFilterConfig {
            &self.config
        }

        fn exchange_name(&self) -> &'static str {
            "mock-exchange"
        }
    }

    fn make_market(id: &str, active: bool, outcomes: usize) -> MarketInfo {
        MarketInfo {
            id: id.to_string(),
            question: format!("Question for {}", id),
            outcomes: (0..outcomes)
                .map(|i| OutcomeInfo {
                    token_id: format!("{}-outcome-{}", id, i),
                    name: format!("Outcome {}", i),
                })
                .collect(),
            active,
        }
    }

    #[test]
    fn filter_is_eligible_checks_active_status() {
        let filter = MockFilter::new(MarketFilterConfig::default());
        let active_market = make_market("m1", true, 2);
        let inactive_market = make_market("m2", false, 2);

        assert!(filter.is_eligible(&active_market));
        assert!(!filter.is_eligible(&inactive_market));
    }

    #[test]
    fn filter_is_eligible_checks_outcome_count() {
        let config = MarketFilterConfig {
            max_outcomes: 3,
            ..Default::default()
        };
        let filter = MockFilter::new(config);

        let binary = make_market("m1", true, 2);
        let three_outcomes = make_market("m2", true, 3);
        let four_outcomes = make_market("m3", true, 4);

        assert!(filter.is_eligible(&binary));
        assert!(filter.is_eligible(&three_outcomes));
        assert!(!filter.is_eligible(&four_outcomes));
    }

    #[test]
    fn filter_default_impl_filters_correctly() {
        let filter = MockFilter::new(MarketFilterConfig::default());
        let markets = vec![
            make_market("m1", true, 2),
            make_market("m2", false, 2),
            make_market("m3", true, 3),
            make_market("m4", true, 100), // Too many outcomes
        ];

        let filtered = filter.filter(&markets);

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].id, "m1");
        assert_eq!(filtered[1].id, "m3");
    }

    #[test]
    fn filter_empty_input_returns_empty() {
        let filter = MockFilter::new(MarketFilterConfig::default());
        let filtered = filter.filter(&[]);
        assert!(filtered.is_empty());
    }

    #[test]
    fn filter_all_eligible_returns_all() {
        let filter = MockFilter::accepting_all(MarketFilterConfig::default());
        let markets = vec![
            make_market("m1", true, 2),
            make_market("m2", false, 2),
            make_market("m3", true, 100),
        ];

        let filtered = filter.filter(&markets);
        assert_eq!(filtered.len(), 3);
    }

    #[test]
    fn filter_none_eligible_returns_empty() {
        let filter = MockFilter::new(MarketFilterConfig::default());
        let markets = vec![
            make_market("m1", false, 2),
            make_market("m2", false, 3),
        ];

        let filtered = filter.filter(&markets);
        assert!(filtered.is_empty());
    }

    #[test]
    fn filter_config_accessor_returns_reference() {
        let config = MarketFilterConfig {
            max_markets: 42,
            ..Default::default()
        };
        let filter = MockFilter::new(config);

        assert_eq!(filter.config().max_markets, 42);
    }

    #[test]
    fn filter_exchange_name_returns_static_str() {
        let filter = MockFilter::new(MarketFilterConfig::default());
        assert_eq!(filter.exchange_name(), "mock-exchange");
    }

    #[test]
    fn filter_trait_is_object_safe() {
        let filter: &dyn MarketFilter = &MockFilter::new(MarketFilterConfig::default());
        assert_eq!(filter.exchange_name(), "mock-exchange");
    }

    #[test]
    fn filter_trait_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MockFilter>();

        fn assert_trait_object_send_sync<T: Send + Sync + ?Sized>() {}
        assert_trait_object_send_sync::<dyn MarketFilter>();
    }
}
