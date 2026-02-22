//! Runtime operator implementation.

use async_trait::async_trait;
use tokio::signal;
#[cfg(feature = "polymarket")]
use tokio::sync::watch;
use tracing::info;

use crate::error::Result;
use crate::infrastructure::config;
#[cfg(feature = "polymarket")]
use crate::infrastructure::orchestration::orchestrator::Orchestrator;
use crate::infrastructure::wallet;
use crate::port::inbound::operator::runtime::{RunRequest, RunStartupSnapshot, RuntimeOperator};

use super::{entry::Operator, shared};

#[async_trait]
impl RuntimeOperator for Operator {
    fn prepare_run(&self, request: &RunRequest) -> Result<RunStartupSnapshot> {
        let config = self.load_run_config(request)?;
        let network = config.network();
        let wallet_display = match wallet::WalletService::wallet_address(&config) {
            Ok(address) if address.len() >= 10 => {
                format!("{}...{}", &address[..6], &address[address.len() - 4..])
            }
            Ok(address) => address,
            Err(_) => "not configured".to_string(),
        };

        Ok(RunStartupSnapshot {
            network_label: shared::network_label(network.environment, network.chain_id),
            chain_id: network.chain_id,
            wallet_display,
            enabled_strategies: config.strategies.enabled,
            dry_run: config.dry_run,
        })
    }

    async fn execute_run(&self, request: RunRequest) -> Result<()> {
        let config = self.load_run_config(&request)?;
        config.init_logging();

        info!(
            chain_id = config.network().chain_id,
            strategies = ?config.strategies.enabled,
            "edgelord starting"
        );

        #[cfg(feature = "polymarket")]
        {
            let (shutdown_tx, shutdown_rx) = watch::channel(false);
            let mut app_handle =
                tokio::spawn(
                    async move { Orchestrator::run_with_shutdown(config, shutdown_rx).await },
                );

            tokio::select! {
                result = &mut app_handle => {
                    shared::map_app_result(result)?;
                    info!("edgelord stopped");
                    return Ok(());
                }
                _ = signal::ctrl_c() => {
                    info!("Shutdown signal received (Ctrl+C)");
                    let _ = shutdown_tx.send(true);
                }
            }

            shared::map_app_result(app_handle.await)?;
        }

        #[cfg(not(feature = "polymarket"))]
        {
            let _ = config;
            info!("No exchange features enabled - exiting");
            tokio::select! {
                _ = signal::ctrl_c() => {
                    info!("Shutdown signal received (Ctrl+C)");
                }
            }
        }

        info!("edgelord stopped");
        Ok(())
    }
}

impl Operator {
    fn load_run_config(&self, request: &RunRequest) -> Result<config::settings::Config> {
        let mut config = config::settings::Config::parse_toml(&request.config_toml)?;
        Self::apply_run_overrides(&mut config, request);
        Ok(config)
    }

    fn apply_run_overrides(config: &mut config::settings::Config, request: &RunRequest) {
        if let Some(chain_id) = request.chain_id {
            match &mut config.exchange_config {
                config::settings::ExchangeSpecificConfig::Polymarket(exchange) => {
                    exchange.chain_id = chain_id;
                }
            }
        }

        if let Some(ref log_level) = request.log_level {
            config.logging.level = log_level.clone();
        }

        if request.json_logs {
            config.logging.format = "json".to_string();
        }

        if let Some(ref strategies) = request.strategies {
            config.strategies.enabled = strategies
                .iter()
                .map(|strategy| Self::normalize_strategy_name(strategy))
                .collect();
        }

        if let Some(min_edge) = request.min_edge {
            config.strategies.single_condition.min_edge = min_edge;
            config.strategies.market_rebalancing.min_edge = min_edge;
        }

        if let Some(min_profit) = request.min_profit {
            config.strategies.single_condition.min_profit = min_profit;
            config.strategies.market_rebalancing.min_profit = min_profit;
            config.risk.min_profit_threshold = min_profit;
        }

        if let Some(max_exposure) = request.max_exposure {
            config.risk.max_total_exposure = max_exposure;
        }

        if let Some(max_position) = request.max_position {
            config.risk.max_position_per_market = max_position;
        }

        if request.telegram_enabled {
            config.telegram.enabled = true;
        }

        if request.dry_run {
            config.dry_run = true;
        }

        if let Some(max_slippage) = request.max_slippage {
            config.risk.max_slippage = max_slippage;
        }

        if let Some(timeout) = request.execution_timeout {
            config.risk.execution_timeout_secs = timeout;
        }

        if let Some(max_markets) = request.max_markets {
            match &mut config.exchange_config {
                config::settings::ExchangeSpecificConfig::Polymarket(exchange) => {
                    exchange.market_filter.max_markets = max_markets;
                }
            }
        }

        if let Some(min_volume) = request.min_volume {
            match &mut config.exchange_config {
                config::settings::ExchangeSpecificConfig::Polymarket(exchange) => {
                    exchange.market_filter.min_volume_24h = min_volume;
                }
            }
        }

        if let Some(min_liquidity) = request.min_liquidity {
            match &mut config.exchange_config {
                config::settings::ExchangeSpecificConfig::Polymarket(exchange) => {
                    exchange.market_filter.min_liquidity = min_liquidity;
                }
            }
        }

        if let Some(max_connections) = request.max_connections {
            config.connection_pool.max_connections = max_connections;
        }

        if let Some(subscriptions) = request.subscriptions_per_connection {
            config.connection_pool.subscriptions_per_connection = subscriptions;
        }

        if let Some(connection_ttl) = request.connection_ttl_seconds {
            config.connection_pool.connection_ttl_secs = connection_ttl;
        }

        if let Some(stats_interval) = request.stats_interval_seconds {
            config.telegram.stats_interval_secs = stats_interval;
        }

        if let Some(ref database_path) = request.database_path {
            config.database = database_path.clone();
        }

        if request.mainnet {
            config.set_mainnet();
        }

        if request.testnet {
            config.set_testnet();
        }
    }

    fn normalize_strategy_name(raw: &str) -> String {
        raw.trim().to_lowercase().replace('-', "_")
    }
}
