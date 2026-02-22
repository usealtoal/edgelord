//! Command-line interface definitions.

use clap::{Parser, Subcommand};
use rust_decimal::Decimal;
use std::path::PathBuf;

use super::paths;
use super::provision::command::ProvisionCommand;

/// Edgelord - Multi-strategy arbitrage detection and execution.
#[derive(Parser, Debug)]
#[command(name = "edgelord")]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Color output mode
    #[arg(long, global = true, default_value = "auto")]
    pub color: ColorChoice,

    /// JSON output for scripting
    #[arg(long, global = true)]
    pub json: bool,

    /// Decrease output verbosity
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Increase output verbosity (-v, -vv, -vvv)
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Clone, Debug, Default, clap::ValueEnum)]
pub enum ColorChoice {
    #[default]
    Auto,
    Always,
    Never,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run the arbitrage detector (foreground, interactive)
    Run(Box<RunArgs>),

    /// Show trading status and statistics
    Status(StatusArgs),

    /// View trading statistics
    #[command(subcommand)]
    Statistics(StatsCommand),

    /// Manage configuration
    #[command(subcommand)]
    Config(ConfigCommand),

    /// Provision exchange-specific configuration
    #[command(subcommand)]
    Provision(ProvisionCommand),

    /// Run diagnostic checks
    #[command(subcommand)]
    Check(CheckCommand),

    /// Manage wallet approvals
    #[command(subcommand)]
    Wallet(WalletCommand),

    /// Initialize configuration interactively
    Init(InitArgs),

    /// Explore available strategies
    #[command(subcommand)]
    Strategies(StrategyCommand),
}

/// Subcommands for `edgelord statistics`
#[derive(Subcommand, Debug)]
pub enum StatsCommand {
    /// Today's statistics (default)
    Today(StatisticsArgs),
    /// Last 7 days
    Week(StatisticsArgs),
    /// Historical view
    History(StatisticsHistoryArgs),
    /// Export stats to CSV
    Export(StatisticsExportArgs),
    /// Prune old records (keeps daily aggregates)
    Prune(StatisticsPruneArgs),
}

/// Subcommands for `edgelord config`
#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    /// Generate a new config file from template
    Init(ConfigInitArgs),
    /// Show effective configuration (with defaults)
    Show(ConfigPathArg),
    /// Validate configuration file
    Validate(ConfigPathArg),
}

/// Subcommands for `edgelord check`
#[derive(Subcommand, Debug)]
pub enum CheckCommand {
    /// Validate configuration file
    Config(ConfigPathArg),
    /// Validate readiness for live trading
    Live(ConfigPathArg),
    /// Run local health checks
    Health(ConfigPathArg),
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
    /// Show wallet address
    Address(ConfigPathArg),
    /// Sweep USDC balance to another address
    Sweep(WalletSweepArgs),
}

/// Subcommands for `edgelord strategies`
#[derive(Subcommand, Debug)]
pub enum StrategyCommand {
    /// List all available strategies
    List,
    /// Explain a specific strategy
    Explain {
        /// Name of the strategy to explain
        name: String,
    },
}

/// Shared argument for commands that only need a config path.
#[derive(Parser, Debug)]
pub struct ConfigPathArg {
    /// Path to configuration file
    #[arg(short, long, default_value_os_t = paths::default_config())]
    pub config: PathBuf,
}

/// Arguments for the `status` subcommand.
#[derive(Parser, Debug)]
pub struct StatusArgs {
    /// Path to database file
    #[arg(long, default_value_os_t = paths::default_database())]
    pub db: PathBuf,

    /// Path to configuration file (for network info)
    #[arg(short, long)]
    pub config: Option<PathBuf>,
}

/// Arguments for the `statistics` subcommand.
#[derive(Parser, Debug)]
pub struct StatisticsArgs {
    /// Path to database file
    #[arg(long, default_value_os_t = paths::default_database())]
    pub db: PathBuf,
}

/// Arguments for `statistics history`.
#[derive(Parser, Debug)]
pub struct StatisticsHistoryArgs {
    /// Number of days to show
    #[arg(default_value = "30")]
    pub days: u32,
    /// Path to database file
    #[arg(long, default_value_os_t = paths::default_database())]
    pub db: PathBuf,
}

/// Arguments for `statistics export`.
#[derive(Parser, Debug)]
pub struct StatisticsExportArgs {
    /// Number of days to export
    #[arg(long, default_value = "30")]
    pub days: u32,
    /// Output file (stdout if not specified)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    /// Path to database file
    #[arg(long, default_value_os_t = paths::default_database())]
    pub db: PathBuf,
}

/// Arguments for `statistics prune`.
#[derive(Parser, Debug)]
pub struct StatisticsPruneArgs {
    /// Keep records newer than this many days
    #[arg(long, default_value = "30")]
    pub days: u32,
    /// Path to database file
    #[arg(long, default_value_os_t = paths::default_database())]
    pub db: PathBuf,
}

/// Arguments for `config init`.
#[derive(Parser, Debug)]
pub struct ConfigInitArgs {
    /// Output path for config file
    #[arg(default_value_os_t = paths::default_config())]
    pub path: PathBuf,
    /// Overwrite if file exists
    #[arg(long)]
    pub force: bool,
}

/// Arguments for the `init` command.
#[derive(Parser, Debug)]
pub struct InitArgs {
    /// Output path for config file
    #[arg(default_value_os_t = paths::default_config())]
    pub path: PathBuf,

    /// Overwrite if file exists
    #[arg(long)]
    pub force: bool,
}

/// Arguments for the `run` subcommand.
#[derive(Parser, Debug)]
pub struct RunArgs {
    /// Path to configuration file
    #[arg(short, long, default_value_os_t = paths::default_config())]
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

    // === Risk Management ===
    /// Override maximum slippage tolerance (0.02 = 2%)
    #[arg(long)]
    pub max_slippage: Option<Decimal>,

    // === Market Discovery ===
    /// Maximum markets to track
    #[arg(long)]
    pub max_markets: Option<usize>,

    /// Minimum 24h volume threshold (USD)
    #[arg(long)]
    pub min_volume: Option<f64>,

    /// Minimum liquidity threshold (USD)
    #[arg(long)]
    pub min_liquidity: Option<f64>,

    // === Connection Pool ===
    /// Maximum WebSocket connections
    #[arg(long)]
    pub max_connections: Option<usize>,

    /// Subscriptions per connection
    #[arg(long)]
    pub subs_per_connection: Option<usize>,

    /// Connection TTL in seconds
    #[arg(long)]
    pub connection_ttl: Option<u64>,

    // === Execution & Runtime ===
    /// Execution timeout in seconds
    #[arg(long)]
    pub execution_timeout: Option<u64>,

    /// Stats update interval in seconds
    #[arg(long)]
    pub stats_interval: Option<u64>,

    /// Path to SQLite database
    #[arg(long)]
    pub database: Option<PathBuf>,

    // === Environment Shortcuts ===
    /// Use mainnet (shortcut for --chain-id=137)
    #[arg(long, conflicts_with = "testnet")]
    pub mainnet: bool,

    /// Use testnet (shortcut for --chain-id=80002)
    #[arg(long, conflicts_with = "mainnet")]
    pub testnet: bool,
}

/// Arguments for the `wallet approve` subcommand.
#[derive(Parser, Debug)]
pub struct WalletApproveArgs {
    /// Path to configuration file
    #[arg(short, long, default_value_os_t = paths::default_config())]
    pub config: PathBuf,

    /// Amount of USDC to approve (in dollars)
    #[arg(long, default_value = "10000")]
    pub amount: Decimal,

    /// Skip confirmation prompt
    #[arg(long)]
    pub yes: bool,
}

/// Arguments for the `wallet sweep` subcommand.
#[derive(Parser, Debug)]
pub struct WalletSweepArgs {
    /// Path to configuration file
    #[arg(short, long, default_value_os_t = paths::default_config())]
    pub config: PathBuf,

    /// Destination address
    #[arg(long)]
    pub to: String,

    /// Asset symbol (default: usdc)
    #[arg(long, default_value = "usdc")]
    pub asset: String,

    /// Network name (default: polygon)
    #[arg(long, default_value = "polygon")]
    pub network: String,

    /// Skip confirmation prompt
    #[arg(long)]
    pub yes: bool,
}
