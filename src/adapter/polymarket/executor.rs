//! Order execution for Polymarket CLOB.

#![allow(dead_code)]

use std::str::FromStr;
use std::sync::Arc;

use alloy_signer_local::PrivateKeySigner;
use async_trait::async_trait;
use polymarket_client_sdk::auth::state::Authenticated;
use polymarket_client_sdk::auth::Normal;
#[allow(unused_imports)]
use polymarket_client_sdk::auth::Signer;
use polymarket_client_sdk::clob::types::response::PostOrderResponse;
use polymarket_client_sdk::clob::types::Side;
use polymarket_client_sdk::clob::{Client, Config as ClobConfig};
use polymarket_client_sdk::types::U256;
use rust_decimal::Decimal;
use tracing::{info, warn};

use crate::app::Config;
use crate::domain::{Opportunity, TokenId};
use crate::error::{ConfigError, ExecutionError, Result};
use crate::exchange::{ExecutionResult, OrderExecutor, OrderId, OrderRequest, OrderSide};

/// Result of executing an arbitrage opportunity (both legs).
#[derive(Debug, Clone)]
pub enum ArbitrageExecutionResult {
    /// Both legs executed successfully.
    Success {
        yes_order_id: String,
        no_order_id: String,
    },
    /// Only one leg executed; the other failed.
    PartialFill {
        filled_leg: TokenId,
        failed_leg: TokenId,
        error: String,
    },
    /// Both legs failed.
    Failed { reason: String },
}

/// Type alias for the authenticated CLOB client.
type AuthenticatedClient = Client<Authenticated<Normal>>;

/// Executes trades on Polymarket CLOB.
pub struct Executor {
    /// The authenticated CLOB client.
    client: Arc<AuthenticatedClient>,
    /// The signer for signing orders.
    signer: Arc<PrivateKeySigner>,
}

impl Executor {
    /// Create new executor by authenticating with Polymarket CLOB.
    pub async fn new(config: &Config) -> Result<Self> {
        let private_key = config
            .wallet
            .private_key
            .as_ref()
            .ok_or(ConfigError::MissingField {
                field: "WALLET_PRIVATE_KEY",
            })?;

        let chain_id = config.network.chain_id;

        // Create signer from private key
        let signer = PrivateKeySigner::from_str(private_key)
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
        let client = Client::new(&config.network.api_url, ClobConfig::default())
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

    /// Execute an arbitrage opportunity by placing orders on both legs.
    pub async fn execute_arbitrage(&self, opportunity: &Opportunity) -> Result<ArbitrageExecutionResult> {
        info!(
            market = %opportunity.market_id(),
            edge = %opportunity.edge(),
            volume = %opportunity.volume(),
            "Executing arbitrage opportunity"
        );

        // Execute both legs in parallel
        let yes_token = opportunity.yes_token().to_string();
        let no_token = opportunity.no_token().to_string();
        let volume = opportunity.volume();
        let yes_price = opportunity.yes_ask();
        let no_price = opportunity.no_ask();

        let (yes_result, no_result) = tokio::join!(
            self.submit_order(&yes_token, Side::Buy, volume, yes_price),
            self.submit_order(&no_token, Side::Buy, volume, no_price),
        );

        match (yes_result, no_result) {
            (Ok(yes_resp), Ok(no_resp)) => {
                info!(
                    yes_order = %yes_resp.order_id,
                    no_order = %no_resp.order_id,
                    "Both legs executed successfully"
                );

                Ok(ArbitrageExecutionResult::Success {
                    yes_order_id: yes_resp.order_id,
                    no_order_id: no_resp.order_id,
                })
            }
            (Ok(yes_resp), Err(no_err)) => {
                warn!(
                    yes_order = %yes_resp.order_id,
                    no_error = %no_err,
                    "NO leg failed, YES leg succeeded"
                );
                Ok(ArbitrageExecutionResult::PartialFill {
                    filled_leg: opportunity.yes_token().clone(),
                    failed_leg: opportunity.no_token().clone(),
                    error: no_err.to_string(),
                })
            }
            (Err(yes_err), Ok(no_resp)) => {
                warn!(
                    no_order = %no_resp.order_id,
                    yes_error = %yes_err,
                    "YES leg failed, NO leg succeeded"
                );
                Ok(ArbitrageExecutionResult::PartialFill {
                    filled_leg: opportunity.no_token().clone(),
                    failed_leg: opportunity.yes_token().clone(),
                    error: yes_err.to_string(),
                })
            }
            (Err(yes_err), Err(no_err)) => {
                warn!(
                    yes_error = %yes_err,
                    no_error = %no_err,
                    "Both legs failed"
                );
                Ok(ArbitrageExecutionResult::Failed {
                    reason: format!("YES: {yes_err}, NO: {no_err}"),
                })
            }
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
        let token_id_u256 = U256::from_str(token_id).map_err(|e| ExecutionError::InvalidTokenId {
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
}

#[async_trait]
impl OrderExecutor for Executor {
    async fn execute(&self, order: &OrderRequest) -> Result<ExecutionResult> {
        let side = match order.side {
            OrderSide::Buy => Side::Buy,
            OrderSide::Sell => Side::Sell,
        };

        match self.submit_order(&order.token_id, side, order.size, order.price).await {
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

    async fn cancel(&self, _order_id: &OrderId) -> Result<()> {
        // TODO: Implement order cancellation via CLOB client
        Err(ExecutionError::OrderRejected("Order cancellation not yet implemented".into()).into())
    }

    fn exchange_name(&self) -> &'static str {
        "Polymarket"
    }
}
