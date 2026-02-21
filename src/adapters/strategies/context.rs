//! Context types for strategy detection.
//!
//! These types provide the necessary information for strategies to
//! analyze markets and detect opportunities.

use rust_decimal::Decimal;

use crate::domain::{Market, MarketId, TokenId};
use crate::ports::DetectionContext as DetectionContextTrait;
use crate::runtime::cache::OrderBookCache;

// Re-export types from ports for consistency
pub use crate::ports::{DetectionResult, MarketContext};

/// Full context for detection including market data.
///
/// This is passed to strategies' `detect()` method.
///
/// Strategies should fail closed when required order books are missing
/// (return no opportunities).
pub struct DetectionContext<'a> {
    /// The market being analyzed.
    pub market: &'a Market,
    /// Order book cache with current prices.
    pub cache: &'a OrderBookCache,
    /// Additional market context.
    market_ctx: MarketContext,
}

impl<'a> DetectionContext<'a> {
    /// Create a new detection context for a market.
    ///
    /// Uses the market's payout and determines the market context
    /// (binary vs multi-outcome) automatically from the market.
    pub fn new(market: &'a Market, cache: &'a OrderBookCache) -> Self {
        let market_ctx = if market.is_binary() {
            MarketContext::binary()
        } else {
            MarketContext::multi_outcome(market.outcome_count())
        };
        Self {
            market,
            cache,
            market_ctx,
        }
    }

    /// Set custom market context.
    #[must_use]
    pub fn with_market_context(mut self, ctx: MarketContext) -> Self {
        self.market_ctx = ctx;
        self
    }

    /// Get the market context.
    #[must_use]
    pub fn market_context(&self) -> MarketContext {
        self.market_ctx.clone()
    }

    /// Get the token IDs from the market's outcomes.
    #[must_use]
    pub fn token_ids(&self) -> Vec<&TokenId> {
        self.market.token_ids()
    }

    /// Get the payout amount from the market.
    #[must_use]
    pub fn payout(&self) -> Decimal {
        self.market.payout()
    }
}

impl<'a> DetectionContextTrait for DetectionContext<'a> {
    fn market_id(&self) -> &MarketId {
        self.market.market_id()
    }

    fn question(&self) -> &str {
        self.market.question()
    }

    fn token_ids(&self) -> Vec<TokenId> {
        self.market.token_ids().into_iter().cloned().collect()
    }

    fn payout(&self) -> Decimal {
        self.market.payout()
    }

    fn market_context(&self) -> MarketContext {
        self.market_ctx.clone()
    }

    fn best_ask(&self, token_id: &TokenId) -> Option<Decimal> {
        self.cache.get(token_id)?.best_ask().map(|l| l.price())
    }

    fn best_bid(&self, token_id: &TokenId) -> Option<Decimal> {
        self.cache.get(token_id)?.best_bid().map(|l| l.price())
    }

    fn ask_volume(&self, token_id: &TokenId) -> Option<Decimal> {
        self.cache.get(token_id)?.best_ask().map(|l| l.size())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_context_binary() {
        let ctx = MarketContext::binary();
        assert!(ctx.is_binary());
        assert!(!ctx.is_multi_outcome());
        assert_eq!(ctx.outcome_count, 2);
        assert!(!ctx.has_dependencies);
    }

    #[test]
    fn test_market_context_multi_outcome() {
        let ctx = MarketContext::multi_outcome(5);
        assert!(!ctx.is_binary());
        assert!(ctx.is_multi_outcome());
        assert_eq!(ctx.outcome_count, 5);
    }

    #[test]
    fn test_market_context_with_dependencies() {
        let deps = vec![MarketId::from("market-1"), MarketId::from("market-2")];
        let ctx = MarketContext::binary().with_dependencies(deps.clone());

        assert!(ctx.has_dependencies);
        assert_eq!(ctx.correlated_markets.len(), 2);
    }

    #[test]
    fn test_detection_result_default() {
        let result = DetectionResult::default();
        assert_eq!(result.opportunity_count, 0);
        assert!(result.solver_state.is_none());
        assert!(result.last_prices.is_empty());
    }

    #[test]
    fn test_detection_context_binary_market() {
        use crate::domain::{Market, Outcome};
        use rust_decimal_macros::dec;

        let outcomes = vec![
            Outcome::new(TokenId::from("yes_token"), "Yes"),
            Outcome::new(TokenId::from("no_token"), "No"),
        ];
        let market = Market::new(
            MarketId::from("market_id"),
            "Test Question",
            outcomes,
            dec!(1),
        );
        let cache = OrderBookCache::new();
        let ctx = DetectionContext::new(&market, &cache);

        assert_eq!(ctx.payout(), Decimal::ONE);
        assert!(ctx.market_context().is_binary());
        assert_eq!(ctx.token_ids().len(), 2);
    }

    #[test]
    fn test_detection_context_custom_payout() {
        use crate::domain::{Market, Outcome};
        use rust_decimal_macros::dec;

        let outcomes = vec![
            Outcome::new(TokenId::from("yes_token"), "Yes"),
            Outcome::new(TokenId::from("no_token"), "No"),
        ];
        let market = Market::new(
            MarketId::from("market_id"),
            "Test Question",
            outcomes,
            dec!(100),
        );
        let cache = OrderBookCache::new();
        let ctx = DetectionContext::new(&market, &cache);

        assert_eq!(ctx.payout(), dec!(100));
    }

    #[test]
    fn test_detection_context_multi_outcome() {
        use crate::domain::{Market, Outcome};
        use rust_decimal_macros::dec;

        let outcomes = vec![
            Outcome::new(TokenId::from("token_1"), "Option A"),
            Outcome::new(TokenId::from("token_2"), "Option B"),
            Outcome::new(TokenId::from("token_3"), "Option C"),
        ];
        let market = Market::new(
            MarketId::from("market_id"),
            "Who will win?",
            outcomes,
            dec!(1),
        );
        let cache = OrderBookCache::new();
        let ctx = DetectionContext::new(&market, &cache);

        assert_eq!(ctx.payout(), Decimal::ONE);
        assert!(ctx.market_context().is_multi_outcome());
        assert_eq!(ctx.token_ids().len(), 3);
    }
}
