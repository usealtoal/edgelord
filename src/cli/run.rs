//! Handler for the `run` command.

use crate::app::Config;
#[cfg(feature = "polymarket")]
use crate::app::App;
use crate::cli::{banner, Cli, RunArgs};
use crate::error::Result;
use tokio::signal;
use tracing::{error, info};

/// Execute the run command.
pub async fn execute(cli: &Cli, args: &RunArgs) -> Result<()> {
    // Load and merge configuration
    let mut config = Config::load(&cli.config)?;

    // Apply CLI overrides
    if let Some(chain_id) = cli.chain_id {
        config.network.chain_id = chain_id;
    }
    if let Some(ref level) = cli.log_level {
        config.logging.level = level.clone();
    }
    if args.json_logs {
        config.logging.format = "json".to_string();
    }
    if let Some(ref strategies) = args.strategies {
        config.strategies.enabled = strategies.split(',').map(|s| s.trim().to_string()).collect();
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
    if cli.dry_run {
        info!("Dry-run mode enabled - will not execute trades");
        // TODO: Wire dry_run into executor
    }

    // Initialize logging
    config.init_logging();

    // Show banner unless disabled
    if !args.no_banner {
        banner::print_banner();
    }

    info!(
        chain_id = config.network.chain_id,
        strategies = ?config.strategies.enabled,
        "edgelord starting"
    );

    // Run the main application
    #[cfg(feature = "polymarket")]
    {
        tokio::select! {
            result = App::run(config) => {
                if let Err(e) = result {
                    error!(error = %e, "Fatal error");
                    std::process::exit(1);
                }
            }
            _ = signal::ctrl_c() => {
                info!("Shutdown signal received");
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
