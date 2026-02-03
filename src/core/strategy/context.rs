//! Context types for strategy detection.
//!
//! These types provide the necessary information for strategies to
//! analyze markets and detect opportunities.

use rust_decimal::Decimal;

use crate::core::domain::{MarketId, MarketPair, OrderBookCache, TokenId};

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
}

impl<'a> DetectionContext<'a> {
    /// Create a new detection context for a binary market pair.
    pub const fn new(pair: &'a MarketPair, cache: &'a OrderBookCache) -> Self {
        Self {
            pair,
            cache,
            market_ctx: MarketContext::binary(),
            token_ids: vec![],
        }
    }

    /// Create a detection context for a multi-outcome market.
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
}
