//! Shared helper routines for operator implementations.

use tracing::error;

use crate::error::{ConfigError, Error, Result};
use crate::infrastructure::config;

pub(super) fn map_app_result(
    result: std::result::Result<Result<()>, tokio::task::JoinError>,
) -> Result<()> {
    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => {
            error!(error = %error, "Application exited with error");
            Err(error)
        }
        Err(error) => {
            error!(error = %error, "Application task join failed");
            Err(Error::Connection(error.to_string()))
        }
    }
}

pub(super) fn validate_sweep_inputs(
    exchange: config::settings::Exchange,
    asset: &str,
    network: &str,
) -> Result<()> {
    let asset_normalized = asset.trim().to_lowercase();
    let network_normalized = network.trim().to_lowercase();

    match exchange {
        config::settings::Exchange::Polymarket => {
            if asset_normalized != "usdc" {
                return Err(ConfigError::InvalidValue {
                    field: "asset",
                    reason: "only usdc is supported for Polymarket sweeps".to_string(),
                }
                .into());
            }
            if network_normalized != "polygon" {
                return Err(ConfigError::InvalidValue {
                    field: "network",
                    reason: "only polygon is supported for Polymarket sweeps".to_string(),
                }
                .into());
            }
        }
    }

    Ok(())
}

pub(super) fn chain_name(chain_id: u64) -> &'static str {
    match chain_id {
        137 => "polygon",
        80002 => "amoy",
        _ => "unknown",
    }
}

pub(super) fn network_label(environment: impl std::fmt::Display, chain_id: u64) -> String {
    format!("{} ({})", environment, chain_name(chain_id))
}

pub(super) fn mask_token(token: &str) -> String {
    if token.len() >= 15 {
        format!("{}...{}", &token[..10], &token[token.len() - 5..])
    } else {
        format!("{}...", &token[..token.len().min(10)])
    }
}
