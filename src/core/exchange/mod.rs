//! Exchange abstraction layer.
//!
//! ## Adding a New Exchange
//!
//! 1. Create a module under `exchange/<name>/`
//! 2. Implement [`ExchangeConfig`] trait with:
//!    - `name()` - Exchange identifier
//!    - `default_payout()` - Payout amount per share
//!    - `binary_outcome_names()` - Names for Yes/No outcomes
//! 3. The default `parse_markets()` implementation handles most cases
//! 4. Add to [`ExchangeFactory`] for runtime selection
//!
//! ## Example
//!
//! ```ignore
//! struct MyExchangeConfig;
//!
//! impl ExchangeConfig for MyExchangeConfig {
//!     fn name(&self) -> &'static str { "myexchange" }
//!     fn default_payout(&self) -> Decimal { dec!(1.00) }
//!     fn binary_outcome_names(&self) -> (&'static str, &'static str) { ("Yes", "No") }
//! }
//! ```

mod factory;
mod filter;
pub mod polymarket;
mod reconnecting;
mod scorer;
mod traits;

pub use factory::ExchangeFactory;
pub use filter::{MarketFilter, MarketFilterConfig};
pub use reconnecting::ReconnectingDataStream;
pub use scorer::MarketScorer;
pub use traits::ExchangeConfig;

// === Trait definitions ===

use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::core::domain::{Opportunity, OrderBook, TokenId};
use crate::error::Error;

/// Unique identifier for an order on an exchange.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrderId(pub String);

impl OrderId {
    /// Create a new `OrderId`.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the underlying ID string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for OrderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Result of attempting to execute an order.
#[derive(Debug, Clone)]
pub enum ExecutionResult {
    /// Order was fully filled.
    Success {
        order_id: OrderId,
        filled_amount: rust_decimal::Decimal,
        average_price: rust_decimal::Decimal,
    },
    /// Order was partially filled.
    PartialFill {
        order_id: OrderId,
        filled_amount: rust_decimal::Decimal,
        remaining_amount: rust_decimal::Decimal,
        average_price: rust_decimal::Decimal,
    },
    /// Order failed to execute.
    Failed { reason: String },
}

impl ExecutionResult {
    /// Check if the execution was successful (fully filled).
    #[must_use]
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// Check if the execution resulted in a partial fill.
    #[must_use]
    pub const fn is_partial(&self) -> bool {
        matches!(self, Self::PartialFill { .. })
    }

    /// Check if the execution failed.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// Get the order ID if available.
    #[must_use]
    pub const fn order_id(&self) -> Option<&OrderId> {
        match self {
            Self::Success { order_id, .. } => Some(order_id),
            Self::PartialFill { order_id, .. } => Some(order_id),
            Self::Failed { .. } => None,
        }
    }
}

/// Represents an order to be executed.
#[derive(Debug, Clone)]
pub struct OrderRequest {
    /// The token/asset ID to trade.
    pub token_id: String,
    /// Buy or Sell.
    pub side: OrderSide,
    /// Order size.
    pub size: Decimal,
    /// Limit price.
    pub price: Decimal,
}

/// Order side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSide {
    /// Buy order.
    Buy,
    /// Sell order.
    Sell,
}

/// Executor for submitting orders to an exchange.
#[async_trait]
pub trait OrderExecutor: Send + Sync {
    /// Execute an order on the exchange.
    async fn execute(&self, order: &OrderRequest) -> Result<ExecutionResult, Error>;

    /// Cancel an existing order.
    async fn cancel(&self, order_id: &OrderId) -> Result<(), Error>;

    /// Get the exchange name for logging/debugging.
    fn exchange_name(&self) -> &'static str;
}

/// Exchange-agnostic market information.
///
/// Represents the minimal information needed to identify and trade a market
/// across different prediction market exchanges.
#[derive(Debug, Clone)]
pub struct MarketInfo {
    /// Unique identifier for the market on the exchange.
    pub id: String,
    /// Human-readable market question or name.
    pub question: String,
    /// Token/outcome identifiers for this market.
    pub outcomes: Vec<OutcomeInfo>,
    /// Whether the market is currently active for trading.
    pub active: bool,
}

/// Information about a single outcome in a market.
#[derive(Debug, Clone)]
pub struct OutcomeInfo {
    /// Token ID for this outcome.
    pub token_id: String,
    /// Human-readable outcome name (e.g., "Yes", "No", "Trump", "Biden").
    pub name: String,
}

impl MarketInfo {
    /// Get all token IDs for this market.
    #[must_use]
    pub fn token_ids(&self) -> Vec<&str> {
        self.outcomes.iter().map(|o| o.token_id.as_str()).collect()
    }

    /// Check if this is a binary (YES/NO) market.
    #[must_use]
    pub fn is_binary(&self) -> bool {
        self.outcomes.len() == 2
    }
}

/// Fetches market information from an exchange.
#[async_trait]
pub trait MarketFetcher: Send + Sync {
    /// Fetch active markets from the exchange.
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of markets to fetch
    async fn get_markets(&self, limit: usize) -> Result<Vec<MarketInfo>, Error>;

    /// Get the exchange name for logging/debugging.
    fn exchange_name(&self) -> &'static str;
}

/// Events received from a market data stream.
#[derive(Debug, Clone)]
pub enum MarketEvent {
    /// Full order book snapshot for a token.
    OrderBookSnapshot { token_id: TokenId, book: OrderBook },
    /// Incremental order book update (deltas).
    OrderBookDelta { token_id: TokenId, book: OrderBook },
    /// Connection established.
    Connected,
    /// Connection lost (may reconnect).
    Disconnected { reason: String },
}

impl MarketEvent {
    /// Get the token ID if this event contains market data.
    #[must_use]
    pub fn token_id(&self) -> Option<&TokenId> {
        match self {
            Self::OrderBookSnapshot { token_id, .. } => Some(token_id),
            Self::OrderBookDelta { token_id, .. } => Some(token_id),
            _ => None,
        }
    }

    /// Get the order book if this event contains one.
    #[must_use]
    pub fn order_book(&self) -> Option<&OrderBook> {
        match self {
            Self::OrderBookSnapshot { book, .. } => Some(book),
            Self::OrderBookDelta { book, .. } => Some(book),
            _ => None,
        }
    }
}

/// Real-time market data stream from an exchange.
///
/// Implementations handle connection management, subscriptions, and message parsing
/// for their specific exchange protocols.
#[async_trait]
pub trait MarketDataStream: Send {
    /// Connect to the exchange's real-time data feed.
    async fn connect(&mut self) -> Result<(), Error>;

    /// Subscribe to market data for the given tokens.
    ///
    /// # Arguments
    ///
    /// * `token_ids` - Token IDs to subscribe to
    async fn subscribe(&mut self, token_ids: &[TokenId]) -> Result<(), Error>;

    /// Receive the next market event.
    ///
    /// This method blocks until an event is available or the connection closes.
    /// Returns `None` when the stream is closed.
    async fn next_event(&mut self) -> Option<MarketEvent>;

    /// Get the exchange name for logging/debugging.
    fn exchange_name(&self) -> &'static str;
}

/// Implement MarketDataStream for boxed trait objects to allow use with generic wrappers.
#[async_trait]
impl MarketDataStream for Box<dyn MarketDataStream> {
    async fn connect(&mut self) -> Result<(), Error> {
        (**self).connect().await
    }

    async fn subscribe(&mut self, token_ids: &[TokenId]) -> Result<(), Error> {
        (**self).subscribe(token_ids).await
    }

    async fn next_event(&mut self) -> Option<MarketEvent> {
        (**self).next_event().await
    }

    fn exchange_name(&self) -> &'static str {
        (**self).exchange_name()
    }
}

/// A successfully executed leg in an arbitrage trade.
#[derive(Debug, Clone)]
pub struct FilledLeg {
    /// Token ID for this leg.
    pub token_id: TokenId,
    /// Order ID returned by exchange.
    pub order_id: String,
}

/// A failed leg in an arbitrage trade.
#[derive(Debug, Clone)]
pub struct FailedLeg {
    /// Token ID for this leg.
    pub token_id: TokenId,
    /// Error message.
    pub error: String,
}

/// Result of executing a multi-leg arbitrage opportunity.
#[derive(Debug, Clone)]
pub enum ArbitrageExecutionResult {
    /// All legs executed successfully.
    Success { filled: Vec<FilledLeg> },
    /// Some legs executed, some failed.
    PartialFill {
        filled: Vec<FilledLeg>,
        failed: Vec<FailedLeg>,
    },
    /// All legs failed.
    Failed { reason: String },
}

impl ArbitrageExecutionResult {
    /// Check if all legs were successful.
    #[must_use]
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// Check if there was a partial fill.
    #[must_use]
    pub const fn is_partial(&self) -> bool {
        matches!(self, Self::PartialFill { .. })
    }

    /// Check if all legs failed.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// Get filled legs if any.
    #[must_use]
    pub fn filled(&self) -> &[FilledLeg] {
        match self {
            Self::Success { filled } => filled,
            Self::PartialFill { filled, .. } => filled,
            Self::Failed { .. } => &[],
        }
    }

    /// Get failed legs if any.
    #[must_use]
    pub fn failed(&self) -> &[FailedLeg] {
        match self {
            Self::Success { .. } => &[],
            Self::PartialFill { failed, .. } => failed,
            Self::Failed { .. } => &[],
        }
    }
}

/// Executor for arbitrage opportunities across multiple legs.
#[async_trait]
pub trait ArbitrageExecutor: Send + Sync {
    /// Execute an arbitrage opportunity by placing orders on all legs.
    async fn execute_arbitrage(
        &self,
        opportunity: &Opportunity,
    ) -> Result<ArbitrageExecutionResult, Error>;

    /// Cancel a specific order by ID.
    async fn cancel(&self, order_id: &OrderId) -> Result<(), Error>;

    /// Get the exchange name for logging/debugging.
    fn exchange_name(&self) -> &'static str;
}
