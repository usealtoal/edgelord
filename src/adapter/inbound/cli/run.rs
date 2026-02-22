//! Handler for the `run` command.

use crate::adapter::inbound::cli::command::RunArgs;
use crate::adapter::inbound::cli::{banner, operator, output};
use crate::error::Result;
use crate::port::inbound::operator::runtime::{RunRequest, RunStartupSnapshot};

/// Execute the run command.
pub async fn execute(args: &RunArgs) -> Result<()> {
    let config_toml = operator::read_config_toml(&args.config)?;
    let machine_output = output::is_json();
    let request = build_run_request(args, config_toml, machine_output);
    let service = operator::operator();

    let styled_output = !request.json_logs;
    if !args.no_banner && styled_output && !output::is_quiet() {
        banner::print_banner();
    }

    if !output::is_quiet() || machine_output {
        let startup = service.prepare_run(&request)?;
        print_startup_config(&startup);
    }

    service.execute_run(request).await
}

fn build_run_request(args: &RunArgs, config_toml: String, force_json_logs: bool) -> RunRequest {
    let strategies = args.strategies.as_ref().map(|raw| {
        raw.split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(ToOwned::to_owned)
            .collect()
    });

    RunRequest {
        config_toml,
        chain_id: args.chain_id,
        log_level: args.log_level.clone(),
        json_logs: args.json_logs || force_json_logs,
        strategies,
        min_edge: args.min_edge,
        min_profit: args.min_profit,
        max_exposure: args.max_exposure,
        max_position: args.max_position,
        telegram_enabled: args.telegram_enabled,
        dry_run: args.dry_run,
        max_slippage: args.max_slippage,
        execution_timeout: args.execution_timeout,
        max_markets: args.max_markets,
        min_volume: args.min_volume,
        min_liquidity: args.min_liquidity,
        max_connections: args.max_connections,
        subscriptions_per_connection: args.subs_per_connection,
        connection_ttl_seconds: args.connection_ttl,
        stats_interval_seconds: args.stats_interval,
        database_path: args
            .database
            .as_ref()
            .map(|path| path.to_string_lossy().to_string()),
        mainnet: args.mainnet,
        testnet: args.testnet,
    }
}

/// Print startup configuration using Astral-style output.
fn print_startup_config(snapshot: &RunStartupSnapshot) {
    let strategies_display = if snapshot.enabled_strategies.is_empty() {
        "none".to_string()
    } else {
        snapshot.enabled_strategies.join(", ")
    };

    output::header(env!("CARGO_PKG_VERSION"));
    output::field("Network", &snapshot.network_label);
    if output::verbosity() > 0 {
        output::field("Chain ID", snapshot.chain_id);
    }
    output::field("Wallet", &snapshot.wallet_display);
    output::field("Strategies", &strategies_display);

    if snapshot.dry_run {
        output::warning("Dry-run mode enabled - trades will be simulated");
    }
}
