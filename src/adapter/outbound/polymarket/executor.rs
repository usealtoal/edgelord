//! Order execution for Polymarket CLOB.

use std::str::FromStr;
use std::sync::Arc;

use alloy_signer_local::PrivateKeySigner;
use async_trait::async_trait;
use polymarket_client_sdk::auth::state::Authenticated;
use polymarket_client_sdk::auth::{Normal, Signer};
use polymarket_client_sdk::clob::types::response::PostOrderResponse;
use polymarket_client_sdk::clob::types::Side;
use polymarket_client_sdk::clob::{Client, Config as ClobConfig};
use polymarket_client_sdk::types::U256;
use rust_decimal::Decimal;
use tracing::{info, warn};

use super::settings::PolymarketRuntimeConfig;
use crate::domain::{
    id::OrderId, opportunity::Opportunity, trade::Failure, trade::Fill, trade::TradeResult,
};
use crate::error::{ConfigError, ExecutionError, Result};
use crate::port::{
    outbound::exchange::ArbitrageExecutor, outbound::exchange::ExecutionResult,
    outbound::exchange::OrderExecutor, outbound::exchange::OrderRequest,
    outbound::exchange::OrderSide,
};

/// Type alias for the authenticated CLOB client.
type AuthenticatedClient = Client<Authenticated<Normal>>;

/// Executes trades on Polymarket CLOB.
pub struct PolymarketExecutor {
    /// The authenticated CLOB client.
    client: Arc<AuthenticatedClient>,
    /// The signer for signing orders.
    signer: Arc<PrivateKeySigner>,
}

impl PolymarketExecutor {
    /// Create new executor by authenticating with Polymarket CLOB.
    pub async fn new(config: &PolymarketRuntimeConfig) -> Result<Self> {
        if config.private_key.trim().is_empty() {
            return Err(ConfigError::MissingField {
                field: "WALLET_PRIVATE_KEY",
            }
            .into());
        }

        let chain_id = config.chain_id;

        // Create signer from private key
        let signer = PrivateKeySigner::from_str(&config.private_key)
            .map_err(|e| ConfigError::InvalidValue {
                field: "WALLET_PRIVATE_KEY",
                reason: e.to_string(),
            })?
            .with_chain_id(Some(chain_id));

        info!(
            chain_id = chain_id,
            address = %signer.address(),
            "Creating CLOB client"
        );

        // Create and authenticate client
        let client = Client::new(&config.api_url, ClobConfig::default())
            .map_err(|e| ExecutionError::AuthFailed(format!("Failed to create CLOB client: {e}")))?
            .authentication_builder(&signer)
            .authenticate()
            .await
            .map_err(|e| ExecutionError::AuthFailed(e.to_string()))?;

        info!("CLOB client authenticated successfully");

        Ok(Self {
            client: Arc::new(client),
            signer: Arc::new(signer),
        })
    }

    /// Execute an arbitrage opportunity by placing orders on all legs in parallel.
    async fn execute_arbitrage_impl(&self, opportunity: &Opportunity) -> Result<TradeResult> {
        info!(
            market = %opportunity.market_id(),
            edge = %opportunity.edge(),
            volume = %opportunity.volume(),
            legs = opportunity.legs().len(),
            "Executing arbitrage opportunity"
        );

        let legs = opportunity.legs();
        if legs.len() < 2 {
            return Ok(TradeResult::Failed {
                reason: "Opportunity must have at least 2 legs".to_string(),
            });
        }

        let volume = opportunity.volume();

        // Execute all legs in parallel
        let futures: Vec<_> = legs
            .iter()
            .map(|leg| {
                let token_id = leg.token_id().clone();
                let token_str = token_id.to_string();
                let price = leg.ask_price();
                async move {
                    let result = self
                        .submit_order(&token_str, Side::Buy, volume, price)
                        .await;
                    (token_id, result)
                }
            })
            .collect();

        let results = futures_util::future::join_all(futures).await;

        // Separate successful and failed legs
        let mut fills = Vec::new();
        let mut failures = Vec::new();

        for (token_id, result) in results {
            match result {
                Ok(resp) => {
                    fills.push(Fill {
                        token_id,
                        order_id: resp.order_id,
                    });
                }
                Err(err) => {
                    failures.push(Failure {
                        token_id,
                        error: err.to_string(),
                    });
                }
            }
        }

        if failures.is_empty() {
            info!(fills = fills.len(), "All legs executed successfully");
            Ok(TradeResult::Success { fills })
        } else if fills.is_empty() {
            let errors: Vec<_> = failures.iter().map(|f| f.error.as_str()).collect();
            warn!(errors = ?errors, "All legs failed");
            Ok(TradeResult::Failed {
                reason: errors.join("; "),
            })
        } else {
            warn!(
                fills = fills.len(),
                failures = failures.len(),
                "Partial fill detected"
            );
            Ok(TradeResult::Partial { fills, failures })
        }
    }

    /// Submit a single order to the CLOB.
    async fn submit_order(
        &self,
        token_id: &str,
        side: Side,
        size: Decimal,
        price: Decimal,
    ) -> Result<PostOrderResponse> {
        // Parse token ID to U256
        let token_id_u256 =
            U256::from_str(token_id).map_err(|e| ExecutionError::InvalidTokenId {
                token_id: token_id.to_string(),
                reason: e.to_string(),
            })?;

        // Build limit order
        let order = self
            .client
            .limit_order()
            .token_id(token_id_u256)
            .side(side)
            .price(price)
            .size(size)
            .build()
            .await
            .map_err(|e| ExecutionError::OrderBuildFailed(e.to_string()))?;

        // Sign order
        let signed_order = self
            .client
            .sign(self.signer.as_ref(), order)
            .await
            .map_err(|e| ExecutionError::SigningFailed(e.to_string()))?;

        // Submit order
        let response = self
            .client
            .post_order(signed_order)
            .await
            .map_err(|e| ExecutionError::SubmissionFailed(e.to_string()))?;

        info!(
            order_id = %response.order_id,
            token_id = token_id,
            side = ?side,
            size = %size,
            price = %price,
            "Order submitted"
        );

        Ok(response)
    }

    /// Cancel an order by ID.
    async fn cancel_order_impl(&self, order_id: &OrderId) -> Result<()> {
        let response = self
            .client
            .cancel_order(order_id.as_str())
            .await
            .map_err(|e| ExecutionError::SubmissionFailed(format!("Cancel failed: {e}")))?;

        if let Some(reason) = response.not_canceled.get(order_id.as_str()) {
            return Err(ExecutionError::OrderRejected(format!(
                "Order {} not cancelled: {}",
                order_id.as_str(),
                reason
            ))
            .into());
        }

        info!(order_id = %order_id, "Order cancelled");
        Ok(())
    }
}

#[async_trait]
impl OrderExecutor for PolymarketExecutor {
    async fn execute(&self, order: &OrderRequest) -> Result<ExecutionResult> {
        let side = match order.side {
            OrderSide::Buy => Side::Buy,
            OrderSide::Sell => Side::Sell,
        };

        match self
            .submit_order(&order.token_id, side, order.size, order.price)
            .await
        {
            Ok(response) => Ok(ExecutionResult::Success {
                order_id: OrderId::new(response.order_id),
                filled_amount: order.size,
                average_price: order.price,
            }),
            Err(e) => Ok(ExecutionResult::Failed {
                reason: e.to_string(),
            }),
        }
    }

    async fn cancel(&self, order_id: &OrderId) -> Result<()> {
        self.cancel_order_impl(order_id).await
    }

    fn exchange_name(&self) -> &'static str {
        "Polymarket"
    }
}

#[async_trait]
impl ArbitrageExecutor for PolymarketExecutor {
    async fn execute_arbitrage(&self, opportunity: &Opportunity) -> Result<TradeResult> {
        self.execute_arbitrage_impl(opportunity).await
    }

    async fn cancel(&self, order_id: &OrderId) -> Result<()> {
        self.cancel_order_impl(order_id).await
    }

    fn exchange_name(&self) -> &'static str {
        "Polymarket"
    }
}
