//! Command-line interface definitions.

pub mod banner;

use clap::{Parser, Subcommand};
use rust_decimal::Decimal;
use std::path::PathBuf;

/// Edgelord - Multi-strategy arbitrage detection and execution.
#[derive(Parser, Debug)]
#[command(name = "edgelord")]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Path to configuration file
    #[arg(short, long, default_value = "config.toml")]
    pub config: PathBuf,

    /// Override chain ID (80002=testnet, 137=mainnet)
    #[arg(long)]
    pub chain_id: Option<u64>,

    /// Override log level (debug, info, warn, error)
    #[arg(long)]
    pub log_level: Option<String>,

    /// Detect opportunities but don't execute trades
    #[arg(long)]
    pub dry_run: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run the arbitrage detector (foreground, interactive)
    Run(RunArgs),

    /// Show service status
    Status,

    /// Tail service logs
    Logs(LogsArgs),

    /// Install systemd service
    Install(InstallArgs),

    /// Uninstall systemd service
    Uninstall,
}

/// Arguments for the `run` subcommand.
#[derive(Parser, Debug)]
pub struct RunArgs {
    /// Skip ASCII art banner
    #[arg(long)]
    pub no_banner: bool,

    /// Use JSON log format instead of pretty
    #[arg(long)]
    pub json_logs: bool,

    /// Comma-separated list of strategies to enable
    #[arg(long)]
    pub strategies: Option<String>,

    /// Override minimum edge threshold
    #[arg(long)]
    pub min_edge: Option<Decimal>,

    /// Override minimum profit threshold
    #[arg(long)]
    pub min_profit: Option<Decimal>,

    /// Override maximum total exposure
    #[arg(long)]
    pub max_exposure: Option<Decimal>,

    /// Override maximum position per market
    #[arg(long)]
    pub max_position: Option<Decimal>,

    /// Enable Telegram notifications
    #[arg(long)]
    pub telegram_enabled: bool,
}

/// Arguments for the `logs` subcommand.
#[derive(Parser, Debug)]
pub struct LogsArgs {
    /// Number of lines to show
    #[arg(short = 'n', long, default_value = "50")]
    pub lines: u32,

    /// Follow log output (like tail -f)
    #[arg(short, long)]
    pub follow: bool,

    /// Show logs since (e.g., "1 hour ago", "2024-01-01")
    #[arg(long)]
    pub since: Option<String>,
}

/// Arguments for the `install` subcommand.
#[derive(Parser, Debug)]
pub struct InstallArgs {
    /// Path to config file for the service
    #[arg(long, default_value = "/opt/edgelord/config.toml")]
    pub config: PathBuf,

    /// User to run the service as
    #[arg(long, default_value = "edgelord")]
    pub user: String,

    /// Working directory for the service
    #[arg(long, default_value = "/opt/edgelord")]
    pub working_dir: PathBuf,
}
