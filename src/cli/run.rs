//! Handler for the `run` command.

#[cfg(feature = "polymarket")]
use crate::app::App;
use crate::app::{Config, Environment, ExchangeSpecificConfig};
use crate::cli::{banner, RunArgs};
use crate::error::{Error, Result};
use tokio::signal;
use tokio::sync::watch;
use tracing::{error, info};

fn map_app_result(result: std::result::Result<Result<()>, tokio::task::JoinError>) -> Result<()> {
    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => {
            error!(error = %e, "Application exited with error");
            Err(e)
        }
        Err(e) => {
            error!(error = %e, "Application task join failed");
            Err(Error::Connection(e.to_string()))
        }
    }
}

/// Execute the run command.
pub async fn execute(args: &RunArgs) -> Result<()> {
    // Load and merge configuration
    let mut config = Config::load(&args.config)?;

    // Apply CLI overrides
    if let Some(chain_id) = args.chain_id {
        match &mut config.exchange_config {
            ExchangeSpecificConfig::Polymarket(poly) => {
                poly.chain_id = chain_id;
            }
        }
    }
    if let Some(ref level) = args.log_level {
        config.logging.level = level.clone();
    }
    if args.json_logs {
        config.logging.format = "json".to_string();
    }
    if let Some(ref strategies) = args.strategies {
        config.strategies.enabled = strategies
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
    }
    if let Some(min_edge) = args.min_edge {
        config.strategies.single_condition.min_edge = min_edge;
        config.strategies.market_rebalancing.min_edge = min_edge;
    }
    if let Some(min_profit) = args.min_profit {
        config.strategies.single_condition.min_profit = min_profit;
        config.strategies.market_rebalancing.min_profit = min_profit;
    }
    if let Some(max_exposure) = args.max_exposure {
        config.risk.max_total_exposure = max_exposure;
    }
    if let Some(max_position) = args.max_position {
        config.risk.max_position_per_market = max_position;
    }
    if args.telegram_enabled {
        config.telegram.enabled = true;
    }
    if args.dry_run {
        config.dry_run = true;
    }

    // Risk overrides
    if let Some(max_slippage) = args.max_slippage {
        config.risk.max_slippage = max_slippage;
    }
    if let Some(timeout) = args.execution_timeout {
        config.risk.execution_timeout_secs = timeout;
    }

    // Market filter overrides
    if let Some(max_markets) = args.max_markets {
        match &mut config.exchange_config {
            ExchangeSpecificConfig::Polymarket(pm) => {
                pm.market_filter.max_markets = max_markets;
            }
        }
    }
    if let Some(min_volume) = args.min_volume {
        match &mut config.exchange_config {
            ExchangeSpecificConfig::Polymarket(pm) => {
                pm.market_filter.min_volume_24h = min_volume;
            }
        }
    }
    if let Some(min_liquidity) = args.min_liquidity {
        match &mut config.exchange_config {
            ExchangeSpecificConfig::Polymarket(pm) => {
                pm.market_filter.min_liquidity = min_liquidity;
            }
        }
    }

    // Connection pool overrides
    if let Some(max_conn) = args.max_connections {
        config.connection_pool.max_connections = max_conn;
    }
    if let Some(subs) = args.subs_per_connection {
        config.connection_pool.subscriptions_per_connection = subs;
    }
    if let Some(ttl) = args.connection_ttl {
        config.connection_pool.connection_ttl_secs = ttl;
    }

    // Runtime overrides
    if let Some(interval) = args.stats_interval {
        config.telegram.stats_interval_secs = interval;
    }
    if let Some(ref db) = args.database {
        config.database = db.to_string_lossy().to_string();
    }

    // Environment shortcuts
    if args.mainnet {
        match &mut config.exchange_config {
            ExchangeSpecificConfig::Polymarket(pm) => {
                pm.chain_id = 137;
                pm.environment = Environment::Mainnet;
            }
        }
    }
    if args.testnet {
        match &mut config.exchange_config {
            ExchangeSpecificConfig::Polymarket(pm) => {
                pm.chain_id = 80002;
                pm.environment = Environment::Testnet;
            }
        }
    }

    // Initialize logging
    config.init_logging();

    // Show banner unless disabled
    if !args.no_banner {
        banner::print_banner();
    }

    info!(
        chain_id = config.network().chain_id,
        strategies = ?config.strategies.enabled,
        "edgelord starting"
    );

    // Run the main application
    #[cfg(feature = "polymarket")]
    {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let mut app_handle =
            tokio::spawn(async move { App::run_with_shutdown(config, shutdown_rx).await });

        tokio::select! {
            result = &mut app_handle => {
                map_app_result(result)?;
                info!("edgelord stopped");
                return Ok(());
            }
            _ = signal::ctrl_c() => {
                info!("Shutdown signal received (Ctrl+C)");
                let _ = shutdown_tx.send(true);
            }
        }

        map_app_result(app_handle.await)?;
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
