//! Wallet operator implementation.

use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::error::Result;
use crate::infrastructure::{config, wallet};
use crate::port::inbound::operator::wallet::{
    ApprovalOutcome, SweepOutcome, WalletApprovalStatus, WalletOperator,
};

use super::{entry::Operator, shared};

#[async_trait]
impl WalletOperator for Operator {
    async fn wallet_status(&self, config_toml: &str) -> Result<WalletApprovalStatus> {
        let config = config::settings::Config::parse_toml(config_toml)?;
        let status = wallet::WalletService::get_approval_status(&config).await?;
        Ok(WalletApprovalStatus {
            exchange: status.exchange,
            wallet_address: status.wallet_address,
            token: status.token,
            allowance: status.allowance,
            spender: status.spender,
            needs_approval: status.needs_approval,
        })
    }

    async fn wallet_approve(&self, config_toml: &str, amount: Decimal) -> Result<ApprovalOutcome> {
        let config = config::settings::Config::parse_toml(config_toml)?;
        let outcome = wallet::WalletService::approve(&config, amount).await?;
        Ok(match outcome {
            wallet::ApprovalOutcome::Approved { tx_hash, amount } => {
                ApprovalOutcome::Approved { tx_hash, amount }
            }
            wallet::ApprovalOutcome::AlreadyApproved { current_allowance } => {
                ApprovalOutcome::AlreadyApproved { current_allowance }
            }
            wallet::ApprovalOutcome::Failed { reason } => ApprovalOutcome::Failed { reason },
        })
    }

    fn wallet_address(&self, config_toml: &str) -> Result<String> {
        let config = config::settings::Config::parse_toml(config_toml)?;
        wallet::WalletService::wallet_address(&config)
    }

    async fn wallet_balance(&self, config_toml: &str) -> Result<Decimal> {
        let config = config::settings::Config::parse_toml(config_toml)?;
        wallet::WalletService::usdc_balance(&config).await
    }

    async fn wallet_sweep(
        &self,
        config_toml: &str,
        to: &str,
        asset: &str,
        network: &str,
    ) -> Result<SweepOutcome> {
        let config = config::settings::Config::parse_toml(config_toml)?;
        shared::validate_sweep_inputs(config.exchange, asset, network)?;

        let outcome = wallet::WalletService::sweep_usdc(&config, to).await?;
        Ok(match outcome {
            wallet::SweepOutcome::NoBalance { balance } => SweepOutcome::NoBalance { balance },
            wallet::SweepOutcome::Transferred { tx_hash, amount } => {
                SweepOutcome::Transferred { tx_hash, amount }
            }
        })
    }
}
