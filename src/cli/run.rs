//! Handler for the `run` command.

#[cfg(feature = "polymarket")]
use crate::app::App;
use crate::app::{Config, ExchangeSpecificConfig};
use crate::cli::{banner, RunArgs};
use crate::error::{Error, Result};
use tokio::signal;
use tokio::sync::watch;
use tracing::{error, info};

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
                match result {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => {
                        error!(error = %e, "Fatal error");
                        return Err(e);
                    }
                    Err(e) => {
                        error!(error = %e, "Application task failed");
                        return Err(Error::Connection(e.to_string()));
                    }
                }
                info!("edgelord stopped");
                return Ok(());
            }
            _ = signal::ctrl_c() => {
                info!("Shutdown signal received");
                let _ = shutdown_tx.send(true);
            }
        }

        match app_handle.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                error!(error = %e, "Fatal error");
                return Err(e);
            }
            Err(e) => {
                error!(error = %e, "Application task failed");
                return Err(Error::Connection(e.to_string()));
            }
        }
    }

    #[cfg(not(feature = "polymarket"))]
    {
        let _ = config;
        info!("No exchange features enabled - exiting");
        tokio::select! {
            _ = signal::ctrl_c() => {
                info!("Shutdown signal received");
            }
        }
    }

    info!("edgelord stopped");
    Ok(())
}
