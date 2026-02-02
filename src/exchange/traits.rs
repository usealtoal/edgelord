//! Exchange trait definitions.
//!
//! These traits define the interface that any exchange implementation must provide.

use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::error::Error;

/// Unique identifier for an order on an exchange.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrderId(pub String);

impl OrderId {
    /// Create a new OrderId.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the underlying ID string.
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
    Failed {
        reason: String,
    },
}

impl ExecutionResult {
    /// Check if the execution was successful (fully filled).
    pub fn is_success(&self) -> bool {
        matches!(self, ExecutionResult::Success { .. })
    }

    /// Check if the execution resulted in a partial fill.
    pub fn is_partial(&self) -> bool {
        matches!(self, ExecutionResult::PartialFill { .. })
    }

    /// Check if the execution failed.
    pub fn is_failed(&self) -> bool {
        matches!(self, ExecutionResult::Failed { .. })
    }

    /// Get the order ID if available.
    pub fn order_id(&self) -> Option<&OrderId> {
        match self {
            ExecutionResult::Success { order_id, .. } => Some(order_id),
            ExecutionResult::PartialFill { order_id, .. } => Some(order_id),
            ExecutionResult::Failed { .. } => None,
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
