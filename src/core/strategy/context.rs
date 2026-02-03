//! Context types for strategy detection.
//!
//! These types provide the necessary information for strategies to
//! analyze markets and detect opportunities.

use rust_decimal::Decimal;

use crate::core::cache::OrderBookCache;
use crate::core::domain::{MarketId, MarketPair, TokenId};

/// Context describing the market being analyzed.
///
/// This provides metadata about the market structure that strategies
/// use to determine applicability.
#[derive(Debug, Clone)]
pub struct MarketContext {
    /// Number of outcomes in the market (2 for binary, 3+ for multi-outcome).
    pub outcome_count: usize,
    /// Whether this market has known dependencies with others.
    pub has_dependencies: bool,
    /// Market IDs of correlated markets (for combinatorial detection).
    pub correlated_markets: Vec<MarketId>,
}

impl MarketContext {
    /// Create context for a simple binary market (YES/NO).
    #[must_use] 
    pub const fn binary() -> Self {
        Self {
            outcome_count: 2,
            has_dependencies: false,
            correlated_markets: vec![],
        }
    }

    /// Create context for a multi-outcome market.
    #[must_use] 
    pub const fn multi_outcome(count: usize) -> Self {
        Self {
            outcome_count: count,
            has_dependencies: false,
            correlated_markets: vec![],
        }
    }

    /// Create context for a market with dependencies.
    #[must_use] 
    pub fn with_dependencies(mut self, markets: Vec<MarketId>) -> Self {
        self.has_dependencies = !markets.is_empty();
        self.correlated_markets = markets;
        self
    }

    /// Check if this is a binary market.
    #[must_use] 
    pub const fn is_binary(&self) -> bool {
        self.outcome_count == 2
    }

    /// Check if this is a multi-outcome market.
    #[must_use] 
    pub const fn is_multi_outcome(&self) -> bool {
        self.outcome_count > 2
    }
}

impl Default for MarketContext {
    fn default() -> Self {
        Self::binary()
    }
}

/// Full context for detection including market data.
///
/// This is passed to strategies' `detect()` method.
pub struct DetectionContext<'a> {
    /// The market pair being analyzed (for binary markets).
    pub pair: &'a MarketPair,
    /// Order book cache with current prices.
    pub cache: &'a OrderBookCache,
    /// Additional market context.
    market_ctx: MarketContext,
    /// Token IDs for multi-outcome markets.
    token_ids: Vec<TokenId>,
    /// Payout amount for the market (defaults to ONE).
    payout: Decimal,
}

impl<'a> DetectionContext<'a> {
    /// Create a new detection context for a binary market pair.
    ///
    /// Defaults payout to `Decimal::ONE`.
    pub const fn new(pair: &'a MarketPair, cache: &'a OrderBookCache) -> Self {
        Self {
            pair,
            cache,
            market_ctx: MarketContext::binary(),
            token_ids: vec![],
            payout: Decimal::ONE,
        }
    }

    /// Create a new detection context with a custom payout.
    pub const fn with_payout(
        pair: &'a MarketPair,
        cache: &'a OrderBookCache,
        payout: Decimal,
    ) -> Self {
        Self {
            pair,
            cache,
            market_ctx: MarketContext::binary(),
            token_ids: vec![],
            payout,
        }
    }

    /// Create a detection context for a multi-outcome market.
    ///
    /// Defaults payout to `Decimal::ONE`.
    pub const fn multi_outcome(
        pair: &'a MarketPair,
        cache: &'a OrderBookCache,
        token_ids: Vec<crate::core::domain::TokenId>,
    ) -> Self {
        let outcome_count = token_ids.len();
        Self {
            pair,
            cache,
            market_ctx: MarketContext::multi_outcome(outcome_count),
            token_ids,
            payout: Decimal::ONE,
        }
    }

    /// Create a detection context for a multi-outcome market with custom payout.
    pub const fn multi_outcome_with_payout(
        pair: &'a MarketPair,
        cache: &'a OrderBookCache,
        token_ids: Vec<crate::core::domain::TokenId>,
        payout: Decimal,
    ) -> Self {
        let outcome_count = token_ids.len();
        Self {
            pair,
            cache,
            market_ctx: MarketContext::multi_outcome(outcome_count),
            token_ids,
            payout,
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

    /// Get the token IDs for multi-outcome markets.
    #[must_use]
    pub fn token_ids(&self) -> &[crate::core::domain::TokenId] {
        &self.token_ids
    }

    /// Get the payout amount for the market.
    #[must_use]
    pub const fn payout(&self) -> Decimal {
        self.payout
    }
}

/// Result from a detection run (for warm-starting).
///
/// Strategies can use this to optimize subsequent detections.
#[derive(Debug, Clone, Default)]
pub struct DetectionResult {
    /// Number of opportunities found.
    pub opportunity_count: usize,
    /// Solver state for warm-starting (opaque bytes).
    pub solver_state: Option<Vec<u8>>,
    /// Last computed prices (for delta detection).
    pub last_prices: Vec<(TokenId, Decimal)>,
}

impl DetectionResult {
    /// Create an empty result.
    #[must_use] 
    pub fn empty() -> Self {
        Self::default()
    }

    /// Create a result with opportunity count.
    #[must_use] 
    pub fn with_count(count: usize) -> Self {
        Self {
            opportunity_count: count,
            ..Default::default()
        }
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
    fn test_detection_context_default_payout() {
        use crate::core::domain::MarketPair;

        let pair = MarketPair::new(
            MarketId::from("market_id"),
            "Test Question",
            TokenId::from("yes_token"),
            TokenId::from("no_token"),
        );
        let cache = OrderBookCache::new();
        let ctx = DetectionContext::new(&pair, &cache);

        assert_eq!(ctx.payout(), Decimal::ONE);
    }

    #[test]
    fn test_detection_context_with_payout() {
        use crate::core::domain::MarketPair;

        let pair = MarketPair::new(
            MarketId::from("market_id"),
            "Test Question",
            TokenId::from("yes_token"),
            TokenId::from("no_token"),
        );
        let cache = OrderBookCache::new();
        let payout = Decimal::new(100, 0); // 100
        let ctx = DetectionContext::with_payout(&pair, &cache, payout);

        assert_eq!(ctx.payout(), payout);
    }

    #[test]
    fn test_detection_context_multi_outcome_default_payout() {
        use crate::core::domain::MarketPair;

        let pair = MarketPair::new(
            MarketId::from("market_id"),
            "Test Question",
            TokenId::from("yes_token"),
            TokenId::from("no_token"),
        );
        let cache = OrderBookCache::new();
        let token_ids = vec![
            TokenId::from("token_1"),
            TokenId::from("token_2"),
            TokenId::from("token_3"),
        ];
        let ctx = DetectionContext::multi_outcome(&pair, &cache, token_ids);

        // Multi-outcome should also default to ONE
        assert_eq!(ctx.payout(), Decimal::ONE);
    }

    #[test]
    fn test_detection_context_multi_outcome_with_payout() {
        use crate::core::domain::MarketPair;

        let pair = MarketPair::new(
            MarketId::from("market_id"),
            "Test Question",
            TokenId::from("yes_token"),
            TokenId::from("no_token"),
        );
        let cache = OrderBookCache::new();
        let token_ids = vec![
            TokenId::from("token_1"),
            TokenId::from("token_2"),
            TokenId::from("token_3"),
        ];
        let payout = Decimal::new(50, 0); // 50
        let ctx = DetectionContext::multi_outcome_with_payout(&pair, &cache, token_ids, payout);

        assert_eq!(ctx.payout(), payout);
    }
}
