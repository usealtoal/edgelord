//! Exchange port for market data and order execution.
//!
//! This module defines the traits for interacting with prediction market
//! exchanges. These are the primary integration points for external services.

use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::domain::{TradeResult, Opportunity, Book, OrderId, TokenId};
use crate::infrastructure::PoolStats;
use crate::error::Error;

/// Result of attempting to execute an order.
#[derive(Debug, Clone)]
pub enum ExecutionResult {
    /// Order was fully filled.
    Success {
        /// The order ID returned by the exchange.
        order_id: OrderId,
        /// Total amount filled.
        filled_amount: Decimal,
        /// Average execution price.
        average_price: Decimal,
    },
    /// Order was partially filled.
    PartialFill {
        /// The order ID returned by the exchange.
        order_id: OrderId,
        /// Amount that was filled.
        filled_amount: Decimal,
        /// Amount still unfilled.
        remaining_amount: Decimal,
        /// Average execution price for filled portion.
        average_price: Decimal,
    },
    /// Order failed to execute.
    Failed {
        /// The failure reason.
        reason: String,
    },
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
    /// Trading volume in the last 24 hours (USD), if available.
    pub volume_24h: Option<f64>,
    /// Current liquidity depth (USD), if available.
    pub liquidity: Option<f64>,
}

/// Information about a single outcome in a market.
#[derive(Debug, Clone)]
pub struct OutcomeInfo {
    /// Token ID for this outcome.
    pub token_id: String,
    /// Human-readable outcome name (e.g., "Yes", "No", "Trump", "Biden").
    pub name: String,
    /// Current price for this outcome (0.0-1.0), if available from REST API.
    pub price: Option<f64>,
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
    BookSnapshot {
        /// The token this order book belongs to.
        token_id: TokenId,
        /// The full order book state.
        book: Book,
    },
    /// Incremental order book update (deltas).
    BookDelta {
        /// The token this update applies to.
        token_id: TokenId,
        /// The order book delta.
        book: Book,
    },
    /// Market has settled (prediction resolved).
    MarketSettled {
        /// The settled market ID.
        market_id: crate::domain::MarketId,
        /// The winning outcome name.
        winning_outcome: String,
        /// Payout amount per share.
        payout_per_share: Decimal,
    },
    /// Connection established.
    Connected,
    /// Connection lost (may reconnect).
    Disconnected {
        /// The disconnection reason.
        reason: String,
    },
}

impl MarketEvent {
    /// Get the token ID if this event contains market data.
    #[must_use]
    pub fn token_id(&self) -> Option<&TokenId> {
        match self {
            Self::BookSnapshot { token_id, .. } => Some(token_id),
            Self::BookDelta { token_id, .. } => Some(token_id),
            _ => None,
        }
    }

    /// Get the order book if this event contains one.
    #[must_use]
    pub fn order_book(&self) -> Option<&Book> {
        match self {
            Self::BookSnapshot { book, .. } => Some(book),
            Self::BookDelta { book, .. } => Some(book),
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

    /// Get connection pool statistics if this stream uses pooling.
    ///
    /// Returns `None` for non-pooled streams.
    fn pool_stats(&self) -> Option<PoolStats> {
        None
    }
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

    fn pool_stats(&self) -> Option<PoolStats> {
        (**self).pool_stats()
    }
}

/// Executor for arbitrage opportunities across multiple legs.
#[async_trait]
pub trait ArbitrageExecutor: Send + Sync {
    /// Execute an arbitrage opportunity by placing orders on all legs.
    async fn execute_arbitrage(
        &self,
        opportunity: &Opportunity,
    ) -> Result<TradeResult, Error>;

    /// Cancel a specific order by ID.
    async fn cancel(&self, order_id: &OrderId) -> Result<(), Error>;

    /// Get the exchange name for logging/debugging.
    fn exchange_name(&self) -> &'static str;
}
