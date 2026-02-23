//! Strategy port for arbitrage detection.
//!
//! Defines the [`Strategy`] trait and supporting types for arbitrage detection
//! algorithms. Strategies analyze market data (order books, prices) and identify
//! profitable trading opportunities.
//!
//! # Overview
//!
//! The strategy system is designed for extensibility:
//!
//! - Implement [`Strategy`] to add new detection algorithms
//! - Use [`DetectionContext`] to access market data during detection
//! - Use [`MarketContext`] to filter which markets a strategy applies to
//!
//! # Example
//!
//! ```ignore
//! use edgelord::port::inbound::strategy::{Strategy, MarketContext, DetectionContext};
//! use edgelord::domain::opportunity::Opportunity;
//!
//! struct MyStrategy;
//!
//! impl Strategy for MyStrategy {
//!     fn name(&self) -> &'static str { "my_strategy" }
//!
//!     fn applies_to(&self, ctx: &MarketContext) -> bool {
//!         ctx.is_binary()
//!     }
//!
//!     fn detect(&self, ctx: &dyn DetectionContext) -> Vec<Opportunity> {
//!         // Detection logic here
//!         vec![]
//!     }
//! }
//! ```

use std::sync::Arc;

use rust_decimal::Decimal;

use crate::domain::{
    book::Book, id::MarketId, id::TokenId, market::Market, market::MarketRegistry,
    opportunity::Opportunity,
};

/// Metadata describing the structure of a market being analyzed.
///
/// Provides information about market structure that strategies use to determine
/// whether they are applicable. For example, a binary arbitrage strategy only
/// applies to markets with exactly two outcomes.
#[derive(Debug, Clone, Default)]
pub struct MarketContext {
    /// Number of outcomes in this market.
    ///
    /// A value of 2 indicates a binary (YES/NO) market. Values greater than 2
    /// indicate multi-outcome markets.
    pub outcome_count: usize,

    /// Whether this market has known dependencies with other markets.
    ///
    /// Set to `true` when logical relations (implication, mutual exclusion, etc.)
    /// have been discovered between this market and others.
    pub has_dependencies: bool,

    /// Identifiers of markets correlated with this one.
    ///
    /// Used by combinatorial detection strategies that analyze multiple
    /// related markets together.
    pub correlated_markets: Vec<MarketId>,
}

impl MarketContext {
    /// Create a context for a simple binary (YES/NO) market.
    ///
    /// Returns a context with `outcome_count` set to 2 and no dependencies.
    #[must_use]
    pub const fn binary() -> Self {
        Self {
            outcome_count: 2,
            has_dependencies: false,
            correlated_markets: vec![],
        }
    }

    /// Create a context for a multi-outcome market.
    ///
    /// # Arguments
    ///
    /// * `count` - Number of outcomes in the market (should be greater than 2).
    #[must_use]
    pub const fn multi_outcome(count: usize) -> Self {
        Self {
            outcome_count: count,
            has_dependencies: false,
            correlated_markets: vec![],
        }
    }

    /// Add dependency information to this context.
    ///
    /// Returns a new context with dependencies set based on the provided
    /// correlated market identifiers.
    ///
    /// # Arguments
    ///
    /// * `markets` - Identifiers of markets correlated with this one.
    #[must_use]
    pub fn with_dependencies(mut self, markets: Vec<MarketId>) -> Self {
        self.has_dependencies = !markets.is_empty();
        self.correlated_markets = markets;
        self
    }

    /// Return `true` if this is a binary (two-outcome) market.
    #[must_use]
    pub const fn is_binary(&self) -> bool {
        self.outcome_count == 2
    }

    /// Return `true` if this is a multi-outcome market (more than two outcomes).
    #[must_use]
    pub const fn is_multi_outcome(&self) -> bool {
        self.outcome_count > 2
    }
}

/// Result from a detection run, used for warm-starting subsequent detections.
///
/// Strategies can return detection results containing state that helps optimize
/// future detection runs. This enables incremental detection and solver warm-starting.
#[derive(Debug, Clone, Default)]
pub struct DetectionResult {
    /// Number of opportunities found in this detection run.
    pub opportunity_count: usize,

    /// Opaque solver state for warm-starting optimization.
    ///
    /// Contains serialized solver state that can be passed to subsequent
    /// detection runs to speed up convergence.
    pub solver_state: Option<Vec<u8>>,

    /// Last computed prices for delta-based detection.
    ///
    /// Strategies can compare current prices against these values to detect
    /// significant changes worth re-analyzing.
    pub last_prices: Vec<(TokenId, Decimal)>,
}

impl DetectionResult {
    /// Create an empty detection result with no opportunities.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Create a detection result with the specified opportunity count.
    ///
    /// # Arguments
    ///
    /// * `count` - Number of opportunities found.
    #[must_use]
    pub fn with_count(count: usize) -> Self {
        Self {
            opportunity_count: count,
            ..Default::default()
        }
    }
}

/// Read-only context providing market data for strategy detection.
///
/// Implementations wrap market metadata and order book caches to provide
/// strategies with the data they need for opportunity detection.
///
/// # Thread Safety
///
/// Implementations must be thread-safe (`Send + Sync`) to support concurrent
/// strategy execution.
pub trait DetectionContext: Send + Sync {
    /// Return the identifier of the market being analyzed.
    fn market_id(&self) -> &MarketId;

    /// Return the human-readable market question.
    fn question(&self) -> &str;

    /// Return the token identifiers for all outcomes in this market.
    fn token_ids(&self) -> Vec<TokenId>;

    /// Return the payout amount for winning outcomes.
    ///
    /// Typically 1.00 for prediction markets where shares pay out $1 on resolution.
    fn payout(&self) -> Decimal;

    /// Return metadata about the market structure.
    ///
    /// Includes outcome count, dependency information, and correlated markets.
    fn market_context(&self) -> MarketContext;

    /// Return the best (lowest) ask price for the specified token.
    ///
    /// # Arguments
    ///
    /// * `token_id` - Token to query.
    ///
    /// Returns `None` if no ask orders exist for this token.
    fn best_ask(&self, token_id: &TokenId) -> Option<Decimal>;

    /// Return the best (highest) bid price for the specified token.
    ///
    /// # Arguments
    ///
    /// * `token_id` - Token to query.
    ///
    /// Returns `None` if no bid orders exist for this token.
    fn best_bid(&self, token_id: &TokenId) -> Option<Decimal>;

    /// Return the available volume at the best ask price for the specified token.
    ///
    /// # Arguments
    ///
    /// * `token_id` - Token to query.
    ///
    /// Returns `None` if no ask orders exist for this token.
    fn ask_volume(&self, token_id: &TokenId) -> Option<Decimal>;

    /// Return the full order book for the specified token.
    ///
    /// # Arguments
    ///
    /// * `token_id` - Token to query.
    ///
    /// Returns `None` if no order book data is available for this token.
    fn order_book(&self, token_id: &TokenId) -> Option<Book>;

    /// Return a reference to the underlying market.
    fn market(&self) -> &Market;
}

/// Arbitrage detection strategy.
///
/// Strategies encapsulate specific detection algorithms for finding profitable
/// trading opportunities. Each strategy can be configured independently and
/// may apply to different market types (binary, multi-outcome, cross-market).
///
/// # Implementation Requirements
///
/// - Strategies must be thread-safe (`Send + Sync`)
/// - The `detect` method should be fast and non-blocking
/// - Use `applies_to` to filter markets before calling `detect`
///
/// # Example
///
/// ```ignore
/// use edgelord::port::inbound::strategy::{Strategy, MarketContext, DetectionContext};
/// use edgelord::domain::opportunity::Opportunity;
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
///         // Detection logic here
///         vec![]
///     }
/// }
/// ```
pub trait Strategy: Send + Sync {
    /// Return the unique identifier for this strategy.
    ///
    /// Used for logging, configuration, and strategy selection.
    fn name(&self) -> &'static str;

    /// Return `true` if this strategy should run for the given market context.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Metadata about the market being considered.
    ///
    /// Strategies should return `false` for markets they cannot analyze
    /// (e.g., a binary arbitrage strategy should reject multi-outcome markets).
    fn applies_to(&self, ctx: &MarketContext) -> bool;

    /// Detect arbitrage opportunities in the provided market context.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Read-only access to market data (prices, order books, etc.).
    ///
    /// Returns a vector of detected opportunities. May return an empty vector
    /// if no opportunities are found.
    fn detect(&self, ctx: &dyn DetectionContext) -> Vec<Opportunity>;

    /// Accept a warm-start hint from a previous detection run.
    ///
    /// # Arguments
    ///
    /// * `previous` - Result from the previous detection run.
    ///
    /// Strategies can use this to optimize subsequent detection runs by
    /// reusing solver state or comparing against previous prices.
    fn warm_start(&mut self, _previous: &DetectionResult) {}

    /// Inject the market registry for cross-market strategies.
    ///
    /// # Arguments
    ///
    /// * `registry` - Shared market registry for accessing related markets.
    ///
    /// Strategies that analyze multiple markets together (combinatorial
    /// arbitrage) can use this to look up correlated markets.
    fn set_market_registry(&mut self, _registry: Arc<MarketRegistry>) {}
}

/// Runtime strategy engine for orchestrating detection across multiple strategies.
///
/// Abstracts over strategy registry implementations so application orchestration
/// code does not depend on concrete adapter types. The engine manages a collection
/// of strategies and runs them against markets in a configured order.
///
/// # Thread Safety
///
/// Implementations must be thread-safe (`Send + Sync`) to support concurrent
/// market analysis.
pub trait StrategyEngine: Send + Sync {
    /// Return the names of configured strategies in evaluation order.
    ///
    /// The order reflects the priority in which strategies are executed.
    fn strategy_names(&self) -> Vec<&'static str>;

    /// Inject the market registry into strategies that require cross-market context.
    ///
    /// # Arguments
    ///
    /// * `registry` - Shared market registry containing all known markets.
    ///
    /// Call this after loading markets to enable combinatorial strategies.
    fn set_market_registry(&mut self, registry: Arc<MarketRegistry>);

    /// Run all applicable strategies and return detected opportunities.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Detection context providing market data.
    ///
    /// Filters strategies using `applies_to`, then calls `detect` on each
    /// applicable strategy. Returns the combined results from all strategies.
    fn detect_opportunities(&self, ctx: &dyn DetectionContext) -> Vec<Opportunity>;
}
