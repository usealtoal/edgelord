//! Command-line interface definitions.
//!
//! Defines the CLI structure for the edgelord application using `clap`.
//! The CLI supports multiple subcommands for running the arbitrage detector,
//! viewing statistics, managing configuration, and performing diagnostic checks.

use clap::{Parser, Subcommand};
use rust_decimal::Decimal;
use std::path::PathBuf;

use super::paths;
use super::provision::command::ProvisionCommand;

/// Root CLI structure for the edgelord application.
///
/// Provides global options (color, JSON output, verbosity) and dispatches
/// to subcommands for specific functionality.
#[derive(Parser, Debug)]
#[command(name = "edgelord")]
#[command(version)]
#[command(about = "A prediction market arbitrage detection and execution CLI")]
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

/// Color output mode for terminal rendering.
///
/// Controls whether ANSI color codes are emitted in terminal output.
#[derive(Clone, Debug, Default, clap::ValueEnum)]
pub enum ColorChoice {
    /// Detect terminal capability automatically.
    #[default]
    Auto,
    /// Always emit color codes.
    Always,
    /// Never emit color codes.
    Never,
}

/// Top-level subcommands for the edgelord CLI.
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

/// Subcommands for `edgelord statistics`.
///
/// Provides views into trading performance over various time ranges,
/// with options to export data and prune old records.
#[derive(Subcommand, Debug)]
pub enum StatsCommand {
    /// Display today's statistics (default view).
    Today(StatisticsArgs),
    /// Display statistics for the last 7 days.
    Week(StatisticsArgs),
    /// Display historical statistics over a configurable period.
    History(StatisticsHistoryArgs),
    /// Export statistics to CSV format.
    Export(StatisticsExportArgs),
    /// Prune old records while keeping daily aggregates.
    Prune(StatisticsPruneArgs),
}

/// Subcommands for `edgelord config`.
///
/// Provides configuration management utilities including generation,
/// display, and validation of configuration files.
#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    /// Generate a new configuration file from template.
    Init(ConfigInitArgs),
    /// Display the effective configuration with defaults applied.
    Show(ConfigPathArg),
    /// Validate a configuration file for correctness.
    Validate(ConfigPathArg),
}

/// Subcommands for `edgelord check`.
///
/// Provides diagnostic commands to verify system readiness before
/// running the arbitrage detector in production.
#[derive(Subcommand, Debug)]
pub enum CheckCommand {
    /// Validate the configuration file syntax and semantics.
    Config(ConfigPathArg),
    /// Validate full readiness for live trading.
    Live(ConfigPathArg),
    /// Run local health checks (disk, memory, dependencies).
    Health(ConfigPathArg),
    /// Test WebSocket connectivity to the exchange.
    Connection(ConfigPathArg),
    /// Test Telegram notification delivery.
    Telegram(ConfigPathArg),
}

/// Subcommands for `edgelord wallet`.
///
/// Provides wallet management utilities including token approvals,
/// status checks, and fund transfers.
#[derive(Subcommand, Debug)]
pub enum WalletCommand {
    /// Approve token spending allowance for the exchange contract.
    Approve(WalletApproveArgs),
    /// Display current wallet approval status and allowances.
    Status(ConfigPathArg),
    /// Display the wallet address derived from the private key.
    Address(ConfigPathArg),
    /// Transfer the full USDC balance to another address.
    Sweep(WalletSweepArgs),
}

/// Subcommands for `edgelord strategies`.
///
/// Provides exploration and documentation of available arbitrage strategies.
#[derive(Subcommand, Debug)]
pub enum StrategyCommand {
    /// List all available arbitrage strategies.
    List,
    /// Display detailed explanation of a specific strategy.
    Explain {
        /// Name of the strategy to explain (e.g., "binary", "multi").
        name: String,
    },
}

/// Shared argument struct for commands that require only a configuration path.
///
/// Provides a reusable argument definition with a default path to the
/// standard configuration file location.
#[derive(Parser, Debug)]
pub struct ConfigPathArg {
    /// Path to the configuration file.
    #[arg(short, long, default_value_os_t = paths::default_config())]
    pub config: PathBuf,
}

/// Arguments for the `status` subcommand.
///
/// Controls data sources for the status display, including the database
/// for trade history and optional configuration for network information.
#[derive(Parser, Debug)]
pub struct StatusArgs {
    /// Path to the SQLite database file.
    #[arg(long, default_value_os_t = paths::default_database())]
    pub db: PathBuf,

    /// Path to configuration file for network information display.
    #[arg(short, long)]
    pub config: Option<PathBuf>,
}

/// Arguments for the `statistics` subcommand.
///
/// Specifies the database source for reading statistics data.
#[derive(Parser, Debug)]
pub struct StatisticsArgs {
    /// Path to the SQLite database file.
    #[arg(long, default_value_os_t = paths::default_database())]
    pub db: PathBuf,
}

/// Arguments for the `statistics history` subcommand.
///
/// Controls the historical view range and data source for statistics.
#[derive(Parser, Debug)]
pub struct StatisticsHistoryArgs {
    /// Number of days of history to display.
    #[arg(default_value = "30")]
    pub days: u32,
    /// Path to the SQLite database file.
    #[arg(long, default_value_os_t = paths::default_database())]
    pub db: PathBuf,
}

/// Arguments for the `statistics export` subcommand.
///
/// Controls CSV export parameters including date range and output destination.
#[derive(Parser, Debug)]
pub struct StatisticsExportArgs {
    /// Number of days of history to export.
    #[arg(long, default_value = "30")]
    pub days: u32,
    /// Output file path (writes to stdout if not specified).
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    /// Path to the SQLite database file.
    #[arg(long, default_value_os_t = paths::default_database())]
    pub db: PathBuf,
}

/// Arguments for the `statistics prune` subcommand.
///
/// Controls record retention during database cleanup operations.
#[derive(Parser, Debug)]
pub struct StatisticsPruneArgs {
    /// Retention period in days (records older than this are pruned).
    #[arg(long, default_value = "30")]
    pub days: u32,
    /// Path to the SQLite database file.
    #[arg(long, default_value_os_t = paths::default_database())]
    pub db: PathBuf,
}

/// Arguments for the `config init` subcommand.
///
/// Controls configuration file generation from the built-in template.
#[derive(Parser, Debug)]
pub struct ConfigInitArgs {
    /// Output path for the generated configuration file.
    #[arg(default_value_os_t = paths::default_config())]
    pub path: PathBuf,
    /// Overwrite the file if it already exists.
    #[arg(long)]
    pub force: bool,
}

/// Arguments for the interactive `init` command.
///
/// Controls the interactive configuration wizard that guides users through
/// initial setup.
#[derive(Parser, Debug)]
pub struct InitArgs {
    /// Output path for the generated configuration file.
    #[arg(default_value_os_t = paths::default_config())]
    pub path: PathBuf,

    /// Overwrite the file if it already exists.
    #[arg(long)]
    pub force: bool,
}

/// Arguments for the `run` subcommand.
///
/// Controls the arbitrage detector runtime including trading parameters,
/// risk limits, market discovery settings, and connection management.
/// All optional fields override the corresponding configuration file values.
#[derive(Parser, Debug)]
pub struct RunArgs {
    /// Path to the configuration file.
    #[arg(short, long, default_value_os_t = paths::default_config())]
    pub config: PathBuf,

    /// Override chain ID (80002 for Amoy testnet, 137 for Polygon mainnet).
    #[arg(long)]
    pub chain_id: Option<u64>,

    /// Override log level (debug, info, warn, error).
    #[arg(long)]
    pub log_level: Option<String>,

    /// Detect opportunities but skip trade execution.
    #[arg(long)]
    pub dry_run: bool,

    /// Skip the ASCII art banner on startup.
    #[arg(long)]
    pub no_banner: bool,

    /// Use JSON log format instead of pretty-printed logs.
    #[arg(long)]
    pub json_logs: bool,

    /// Comma-separated list of strategies to enable (e.g., "binary,multi").
    #[arg(long)]
    pub strategies: Option<String>,

    /// Override minimum edge threshold for opportunity detection.
    #[arg(long)]
    pub min_edge: Option<Decimal>,

    /// Override minimum expected profit threshold (USD).
    #[arg(long)]
    pub min_profit: Option<Decimal>,

    /// Override maximum total portfolio exposure (USD).
    #[arg(long)]
    pub max_exposure: Option<Decimal>,

    /// Override maximum position size per market (USD).
    #[arg(long)]
    pub max_position: Option<Decimal>,

    /// Enable Telegram notifications for trade events.
    #[arg(long)]
    pub telegram_enabled: bool,

    // === Risk Management ===
    /// Override maximum slippage tolerance (e.g., 0.02 for 2%).
    #[arg(long)]
    pub max_slippage: Option<Decimal>,

    // === Market Discovery ===
    /// Maximum number of markets to track simultaneously.
    #[arg(long)]
    pub max_markets: Option<usize>,

    /// Minimum 24-hour volume threshold in USD.
    #[arg(long)]
    pub min_volume: Option<f64>,

    /// Minimum liquidity depth threshold in USD.
    #[arg(long)]
    pub min_liquidity: Option<f64>,

    // === Connection Pool ===
    /// Maximum number of WebSocket connections in the pool.
    #[arg(long)]
    pub max_connections: Option<usize>,

    /// Maximum subscriptions per WebSocket connection.
    #[arg(long)]
    pub subs_per_connection: Option<usize>,

    /// Connection time-to-live in seconds before refresh.
    #[arg(long)]
    pub connection_ttl: Option<u64>,

    // === Execution & Runtime ===
    /// Trade execution timeout in seconds.
    #[arg(long)]
    pub execution_timeout: Option<u64>,

    /// Statistics update interval in seconds.
    #[arg(long)]
    pub stats_interval: Option<u64>,

    /// Path to the SQLite database for persistence.
    #[arg(long)]
    pub database: Option<PathBuf>,

    // === Environment Shortcuts ===
    /// Use Polygon mainnet (shortcut for chain ID 137).
    #[arg(long, conflicts_with = "testnet")]
    pub mainnet: bool,

    /// Use Amoy testnet (shortcut for chain ID 80002).
    #[arg(long, conflicts_with = "mainnet")]
    pub testnet: bool,
}

/// Arguments for the `wallet approve` subcommand.
///
/// Controls ERC-20 token approval for the exchange contract.
#[derive(Parser, Debug)]
pub struct WalletApproveArgs {
    /// Path to the configuration file.
    #[arg(short, long, default_value_os_t = paths::default_config())]
    pub config: PathBuf,

    /// Amount of USDC to approve in dollars.
    #[arg(long, default_value = "10000")]
    pub amount: Decimal,

    /// Skip the confirmation prompt.
    #[arg(long)]
    pub yes: bool,
}

/// Arguments for the `wallet sweep` subcommand.
///
/// Controls fund transfer from the trading wallet to an external address.
#[derive(Parser, Debug)]
pub struct WalletSweepArgs {
    /// Path to the configuration file.
    #[arg(short, long, default_value_os_t = paths::default_config())]
    pub config: PathBuf,

    /// Destination address (0x-prefixed Ethereum address).
    #[arg(long)]
    pub to: String,

    /// Asset symbol to transfer (default: usdc).
    #[arg(long, default_value = "usdc")]
    pub asset: String,

    /// Network name for the transfer (default: polygon).
    #[arg(long, default_value = "polygon")]
    pub network: String,

    /// Skip the confirmation prompt.
    #[arg(long)]
    pub yes: bool,
}
