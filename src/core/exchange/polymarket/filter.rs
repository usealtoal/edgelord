//! Market filter for Polymarket exchange.
//!
//! Implements [`MarketFilter`] to filter Polymarket markets for subscription eligibility.

use crate::app::PolymarketFilterConfig;
use crate::core::exchange::{MarketFilter, MarketFilterConfig, MarketInfo};

/// Filter for Polymarket markets.
///
/// Determines which markets are eligible for subscription based on
/// configurable criteria like outcome count and market type.
#[derive(Debug, Clone)]
pub struct PolymarketFilter {
    /// Filter configuration converted to domain model.
    config: MarketFilterConfig,
}

impl PolymarketFilter {
    /// Create a new Polymarket filter from configuration.
    #[must_use]
    pub fn new(config: &PolymarketFilterConfig) -> Self {
        Self {
            config: MarketFilterConfig {
                max_markets: config.max_markets,
                max_subscriptions: config.max_subscriptions,
                min_volume_24h: config.min_volume_24h,
                min_liquidity: config.min_liquidity,
                max_spread_pct: config.max_spread_pct,
                include_binary: config.include_binary,
                include_multi_outcome: config.include_multi_outcome,
                max_outcomes: config.max_outcomes,
            },
        }
    }
}

impl MarketFilter for PolymarketFilter {
    fn is_eligible(&self, market: &MarketInfo) -> bool {
        // Must be active
        if !market.active {
            return false;
        }

        let outcome_count = market.outcomes.len();

        // Must not exceed max outcomes
        if outcome_count > self.config.max_outcomes {
            return false;
        }

        // Check binary/multi-outcome inclusion
        let is_binary = outcome_count == 2;

        if is_binary && !self.config.include_binary {
            return false;
        }

        if !is_binary && !self.config.include_multi_outcome {
            return false;
        }

        // TODO: Add volume and liquidity filtering when API data is available
        // The current Polymarket API response (PolymarketMarket) doesn't include
        // volume_24h or liquidity fields. To implement these filters:
        // 1. Check if the API has endpoints that provide volume/liquidity data
        //    (e.g., market stats endpoint or enhanced market data)
        // 2. Add volume_24h and liquidity fields to PolymarketMarket struct
        // 3. Pipe the data through to MarketInfo (add optional fields)
        // 4. Add filter checks here:
        //    - if let Some(volume) = market.volume_24h {
        //          if volume < self.config.min_volume_24h { return false; }
        //      }
        //    - if let Some(liquidity) = market.liquidity {
        //          if liquidity < self.config.min_liquidity { return false; }
        //      }
        //
        // Currently min_volume_24h and min_liquidity config values are ignored.

        true
    }

    fn config(&self) -> &MarketFilterConfig {
        &self.config
    }

    fn exchange_name(&self) -> &'static str {
        "polymarket"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::exchange::OutcomeInfo;

    fn default_config() -> PolymarketFilterConfig {
        PolymarketFilterConfig::default()
    }

    fn make_market(id: &str, active: bool, outcome_count: usize) -> MarketInfo {
        let outcomes: Vec<OutcomeInfo> = (0..outcome_count)
            .map(|i| OutcomeInfo {
                token_id: format!("token-{}", i),
                name: format!("Outcome {}", i),
                price: None,
            })
            .collect();

        MarketInfo {
            id: id.to_string(),
            question: format!("Test market {}", id),
            outcomes,
            active,
        }
    }

    // --- Constructor tests ---

    #[test]
    fn new_creates_filter_with_config_values() {
        let config = default_config();
        let filter = PolymarketFilter::new(&config);

        assert_eq!(filter.config.max_markets, config.max_markets);
        assert_eq!(filter.config.max_subscriptions, config.max_subscriptions);
        assert!((filter.config.min_volume_24h - config.min_volume_24h).abs() < f64::EPSILON);
        assert!((filter.config.min_liquidity - config.min_liquidity).abs() < f64::EPSILON);
        assert!((filter.config.max_spread_pct - config.max_spread_pct).abs() < f64::EPSILON);
        assert_eq!(filter.config.include_binary, config.include_binary);
        assert_eq!(
            filter.config.include_multi_outcome,
            config.include_multi_outcome
        );
        assert_eq!(filter.config.max_outcomes, config.max_outcomes);
    }

    #[test]
    fn new_with_custom_config() {
        let mut config = default_config();
        config.max_markets = 100;
        config.max_subscriptions = 50;
        config.min_volume_24h = 5000.0;
        config.min_liquidity = 2000.0;
        config.max_spread_pct = 0.05;
        config.include_binary = false;
        config.include_multi_outcome = true;
        config.max_outcomes = 5;

        let filter = PolymarketFilter::new(&config);

        assert_eq!(filter.config.max_markets, 100);
        assert_eq!(filter.config.max_subscriptions, 50);
        assert!((filter.config.min_volume_24h - 5000.0).abs() < f64::EPSILON);
        assert!((filter.config.min_liquidity - 2000.0).abs() < f64::EPSILON);
        assert!((filter.config.max_spread_pct - 0.05).abs() < f64::EPSILON);
        assert!(!filter.config.include_binary);
        assert!(filter.config.include_multi_outcome);
        assert_eq!(filter.config.max_outcomes, 5);
    }

    // --- is_eligible tests ---

    #[test]
    fn is_eligible_active_binary_market() {
        let filter = PolymarketFilter::new(&default_config());
        let market = make_market("m1", true, 2);

        assert!(filter.is_eligible(&market));
    }

    #[test]
    fn is_eligible_inactive_market_returns_false() {
        let filter = PolymarketFilter::new(&default_config());
        let market = make_market("m1", false, 2);

        assert!(!filter.is_eligible(&market));
    }

    #[test]
    fn is_eligible_exceeds_max_outcomes_returns_false() {
        let mut config = default_config();
        config.max_outcomes = 5;
        let filter = PolymarketFilter::new(&config);

        let market = make_market("m1", true, 6);

        assert!(!filter.is_eligible(&market));
    }

    #[test]
    fn is_eligible_at_max_outcomes_returns_true() {
        let mut config = default_config();
        config.max_outcomes = 5;
        let filter = PolymarketFilter::new(&config);

        let market = make_market("m1", true, 5);

        assert!(filter.is_eligible(&market));
    }

    #[test]
    fn is_eligible_binary_excluded_returns_false() {
        let mut config = default_config();
        config.include_binary = false;
        let filter = PolymarketFilter::new(&config);

        let market = make_market("m1", true, 2);

        assert!(!filter.is_eligible(&market));
    }

    #[test]
    fn is_eligible_multi_outcome_excluded_returns_false() {
        let mut config = default_config();
        config.include_multi_outcome = false;
        let filter = PolymarketFilter::new(&config);

        let market = make_market("m1", true, 4);

        assert!(!filter.is_eligible(&market));
    }

    #[test]
    fn is_eligible_multi_outcome_when_binary_excluded() {
        let mut config = default_config();
        config.include_binary = false;
        let filter = PolymarketFilter::new(&config);

        let market = make_market("m1", true, 4);

        assert!(filter.is_eligible(&market));
    }

    #[test]
    fn is_eligible_binary_when_multi_excluded() {
        let mut config = default_config();
        config.include_multi_outcome = false;
        let filter = PolymarketFilter::new(&config);

        let market = make_market("m1", true, 2);

        assert!(filter.is_eligible(&market));
    }

    #[test]
    fn is_eligible_single_outcome_treated_as_multi() {
        let mut config = default_config();
        config.include_multi_outcome = false;
        let filter = PolymarketFilter::new(&config);

        // Single outcome (outcome_count != 2) is treated as multi-outcome
        let market = make_market("m1", true, 1);

        assert!(!filter.is_eligible(&market));
    }

    #[test]
    fn is_eligible_zero_outcomes_treated_as_multi() {
        let mut config = default_config();
        config.include_multi_outcome = false;
        let filter = PolymarketFilter::new(&config);

        // Zero outcomes (outcome_count != 2) is treated as multi-outcome
        let market = make_market("m1", true, 0);

        assert!(!filter.is_eligible(&market));
    }

    // --- filter (batch) tests ---

    #[test]
    fn filter_returns_only_eligible_markets() {
        let filter = PolymarketFilter::new(&default_config());
        let markets = vec![
            make_market("m1", true, 2),
            make_market("m2", false, 2),
            make_market("m3", true, 4),
            make_market("m4", true, 100), // Exceeds max_outcomes
        ];

        let filtered = filter.filter(&markets);

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].id, "m1");
        assert_eq!(filtered[1].id, "m3");
    }

    #[test]
    fn filter_empty_input_returns_empty() {
        let filter = PolymarketFilter::new(&default_config());

        let filtered = filter.filter(&[]);

        assert!(filtered.is_empty());
    }

    #[test]
    fn filter_all_eligible_returns_all() {
        let filter = PolymarketFilter::new(&default_config());
        let markets = vec![
            make_market("m1", true, 2),
            make_market("m2", true, 3),
            make_market("m3", true, 5),
        ];

        let filtered = filter.filter(&markets);

        assert_eq!(filtered.len(), 3);
    }

    #[test]
    fn filter_none_eligible_returns_empty() {
        let filter = PolymarketFilter::new(&default_config());
        let markets = vec![make_market("m1", false, 2), make_market("m2", false, 3)];

        let filtered = filter.filter(&markets);

        assert!(filtered.is_empty());
    }

    // --- config accessor tests ---

    #[test]
    fn config_returns_reference() {
        let mut pconfig = default_config();
        pconfig.max_markets = 42;
        let filter = PolymarketFilter::new(&pconfig);

        assert_eq!(filter.config().max_markets, 42);
    }

    #[test]
    fn config_accessor_returns_converted_config() {
        let filter = PolymarketFilter::new(&default_config());
        let config = filter.config();

        // Verify it's a MarketFilterConfig with expected values
        assert_eq!(config.max_markets, 500); // PolymarketFilterConfig default
        assert_eq!(config.max_subscriptions, 2000); // PolymarketFilterConfig default
    }

    // --- exchange_name tests ---

    #[test]
    fn exchange_name_returns_polymarket() {
        let filter = PolymarketFilter::new(&default_config());

        assert_eq!(filter.exchange_name(), "polymarket");
    }

    // --- trait object tests ---

    #[test]
    fn filter_trait_is_object_safe() {
        let filter: &dyn MarketFilter = &PolymarketFilter::new(&default_config());
        assert_eq!(filter.exchange_name(), "polymarket");
    }

    #[test]
    fn filter_trait_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PolymarketFilter>();
    }

    // --- Debug and Clone tests ---

    #[test]
    fn filter_implements_debug() {
        let filter = PolymarketFilter::new(&default_config());
        let debug_str = format!("{:?}", filter);
        assert!(debug_str.contains("PolymarketFilter"));
    }

    #[test]
    fn filter_implements_clone() {
        let filter = PolymarketFilter::new(&default_config());
        let cloned = filter.clone();
        assert_eq!(filter.config().max_markets, cloned.config().max_markets);
    }
}
