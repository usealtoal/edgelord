//! Strategy port for arbitrage detection algorithms.
//!
//! This module defines the trait that detection strategies must implement.
//! Strategies are responsible for finding arbitrage opportunities in markets.

use std::sync::Arc;

use crate::domain::{MarketId, MarketRegistry, Opportunity, TokenId};
use rust_decimal::Decimal;

/// Context describing the market being analyzed.
///
/// This provides metadata about the market structure that strategies
/// use to determine applicability.
#[derive(Debug, Clone, Default)]
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

/// Full context for detection including market data.
///
/// This is passed to strategies' `detect()` method.
/// Contains all information a strategy needs to analyze a market.
pub trait DetectionContext: Send + Sync {
    /// Get the market ID being analyzed.
    fn market_id(&self) -> &MarketId;

    /// Get the market question.
    fn question(&self) -> &str;

    /// Get the token IDs for this market's outcomes.
    fn token_ids(&self) -> Vec<TokenId>;

    /// Get the payout amount for this market.
    fn payout(&self) -> Decimal;

    /// Get the market context (outcome count, dependencies, etc.).
    fn market_context(&self) -> MarketContext;

    /// Get the best ask price for a token, if available.
    fn best_ask(&self, token_id: &TokenId) -> Option<Decimal>;

    /// Get the best bid price for a token, if available.
    fn best_bid(&self, token_id: &TokenId) -> Option<Decimal>;

    /// Get available volume at the best ask for a token.
    fn ask_volume(&self, token_id: &TokenId) -> Option<Decimal>;
}

/// A detection strategy that finds arbitrage opportunities.
///
/// Strategies encapsulate specific detection algorithms. Each strategy
/// can be configured independently and may apply to different market types.
///
/// # Implementation Notes
///
/// - Strategies must be thread-safe (`Send + Sync`)
/// - The `detect` method should be pure and idempotent
/// - Use `warm_start` for iterative optimization algorithms
#[allow(dead_code)] // Extension point for custom strategies
pub trait Strategy: Send + Sync {
    /// Unique identifier for this strategy.
    ///
    /// Used in configuration and logging.
    fn name(&self) -> &'static str;

    /// Check if this strategy should run for a given market context.
    ///
    /// For example, single-condition only applies to binary markets,
    /// while market rebalancing applies to multi-outcome markets.
    fn applies_to(&self, ctx: &MarketContext) -> bool;

    /// Detect opportunities given current market state.
    ///
    /// Returns all found opportunities (may be empty).
    /// The concrete DetectionContext provides access to market data.
    fn detect(&self, ctx: &dyn DetectionContext) -> Vec<Opportunity>;

    /// Optional: provide warm-start hint from previous detection.
    ///
    /// Strategies can use this to speed up iterative algorithms
    /// (e.g., Frank-Wolfe can reuse previous solution).
    fn warm_start(&mut self, _previous: &DetectionResult) {}

    /// Optional: inject the market registry for strategies that need it.
    ///
    /// Called by the orchestrator after the registry is built. Strategies
    /// that don't need it can ignore this (default no-op).
    fn set_market_registry(&mut self, _registry: Arc<MarketRegistry>) {}
}
