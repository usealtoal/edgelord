//! Strategy port for arbitrage detection.
//!
//! This module defines the Strategy trait and supporting types for
//! detection algorithms. Strategies analyze market data and find
//! arbitrage opportunities.

use std::sync::Arc;

use rust_decimal::Decimal;

use crate::domain::{
    book::Book, id::MarketId, id::TokenId, market::Market, market::MarketRegistry,
    opportunity::Opportunity,
};

/// Context describing the market being analyzed.
///
/// Provides metadata about market structure that strategies use
/// to determine applicability.
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

/// Context for strategy detection.
///
/// Provides read-only access to market data needed for detection.
/// Implementations typically wrap a Market and BookCache.
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

    /// Get the full order book for a token, if available.
    fn order_book(&self, token_id: &TokenId) -> Option<Book>;

    /// Get the underlying market reference.
    fn market(&self) -> &Market;
}

/// A detection strategy that finds arbitrage opportunities.
///
/// Strategies encapsulate specific detection algorithms. Each strategy
/// can be configured independently and may apply to different market types.
///
/// # Implementing a Strategy
///
/// ```ignore
/// use edgelord::port::{inbound::strategy::Strategy, inbound::strategy::MarketContext, inbound::strategy::DetectionContext};
/// use edgelord::domain::Opportunity;
///
/// pub struct MyStrategy;
///
/// impl Strategy for MyStrategy {
///     fn name(&self) -> &'static str { "my_strategy" }
///
///     fn applies_to(&self, ctx: &MarketContext) -> bool {
///         ctx.is_binary()
///     }
///
///     fn detect(&self, ctx: &dyn DetectionContext) -> Vec<Opportunity> {
///         // Your detection logic
///         vec![]
///     }
/// }
/// ```
pub trait Strategy: Send + Sync {
    /// Unique identifier for this strategy.
    fn name(&self) -> &'static str;

    /// Check if this strategy should run for a given market context.
    fn applies_to(&self, ctx: &MarketContext) -> bool;

    /// Detect opportunities given current market state.
    fn detect(&self, ctx: &dyn DetectionContext) -> Vec<Opportunity>;

    /// Optional: provide warm-start hint from previous detection.
    fn warm_start(&mut self, _previous: &DetectionResult) {}

    /// Optional: inject the market registry for strategies that need it.
    fn set_market_registry(&mut self, _registry: Arc<MarketRegistry>) {}
}

/// Runtime strategy engine used by application services.
///
/// This abstracts over strategy registry implementations so application
/// orchestration code does not depend on concrete adapter types.
pub trait StrategyEngine: Send + Sync {
    /// Names of configured strategies in evaluation order.
    fn strategy_names(&self) -> Vec<&'static str>;

    /// Inject market registry into strategies that need cross-market context.
    fn set_market_registry(&mut self, registry: Arc<MarketRegistry>);

    /// Run all applicable strategies for the provided detection context.
    fn detect_opportunities(&self, ctx: &dyn DetectionContext) -> Vec<Opportunity>;
}
