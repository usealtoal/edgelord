//! Command-line interface definitions.

pub mod banner;
pub mod check;
pub mod logs;
pub mod run;
pub mod service;
pub mod status;
pub mod wallet;

use clap::{Parser, Subcommand};
use rust_decimal::Decimal;
use std::path::PathBuf;

/// Edgelord - Multi-strategy arbitrage detection and execution.
#[derive(Parser, Debug)]
#[command(name = "edgelord")]
#[command(version, about, long_about = None)]
pub struct Cli {
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

    /// Manage systemd service
    #[command(subcommand)]
    Service(ServiceCommand),

    /// Run diagnostic checks
    #[command(subcommand)]
    Check(CheckCommand),

    /// Manage wallet approvals
    #[command(subcommand)]
    Wallet(WalletCommand),
}

/// Subcommands for `edgelord service`
#[derive(Subcommand, Debug)]
pub enum ServiceCommand {
    /// Install systemd service
    Install(InstallArgs),
    /// Uninstall systemd service
    Uninstall,
}

/// Subcommands for `edgelord check`
#[derive(Subcommand, Debug)]
pub enum CheckCommand {
    /// Validate configuration file
    Config(ConfigPathArg),
    /// Test WebSocket connection to exchange
    Connection(ConfigPathArg),
    /// Test Telegram notification setup
    Telegram(ConfigPathArg),
}

/// Subcommands for `edgelord wallet`
#[derive(Subcommand, Debug)]
pub enum WalletCommand {
    /// Approve token spending for exchange
    Approve(WalletApproveArgs),
    /// Show wallet approval status
    Status(ConfigPathArg),
}

/// Shared argument for commands that only need a config path.
#[derive(Parser, Debug)]
pub struct ConfigPathArg {
    /// Path to configuration file
    #[arg(short, long, default_value = "config.toml")]
    pub config: PathBuf,
}

/// Arguments for the `run` subcommand.
#[derive(Parser, Debug)]
pub struct RunArgs {
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

/// Arguments for the `wallet approve` subcommand.
#[derive(Parser, Debug)]
pub struct WalletApproveArgs {
    /// Path to configuration file
    #[arg(short, long, default_value = "config.toml")]
    pub config: PathBuf,

    /// Amount of USDC to approve (in dollars)
    #[arg(long, default_value = "10000")]
    pub amount: Decimal,

    /// Skip confirmation prompt
    #[arg(long)]
    pub yes: bool,
}
