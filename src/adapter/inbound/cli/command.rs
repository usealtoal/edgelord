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

/// Prediction market arbitrage detection and execution CLI
#[derive(Parser, Debug)]
#[command(name = "edgelord")]
#[command(version)]
pub struct Cli {
    /// Color output mode [auto, always, never]
    #[arg(
        long,
        global = true,
        default_value = "auto",
        hide_possible_values = true
    )]
    pub color: ColorChoice,

    /// JSON output for scripting
    #[arg(long, global = true)]
    pub json: bool,

    /// Decrease output verbosity
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Increase output verbosity
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Commands,
}

/// Color output mode for terminal rendering.
#[derive(Clone, Debug, Default, clap::ValueEnum)]
pub enum ColorChoice {
    /// Detect automatically
    #[default]
    Auto,
    /// Always use colors
    Always,
    /// Never use colors
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    // Tests for CLI structure validation

    #[test]
    fn test_cli_command_factory_builds() {
        // Verifies that the CLI definition is valid
        let _ = Cli::command();
    }

    #[test]
    fn test_cli_has_version() {
        let cmd = Cli::command();
        assert!(cmd.get_version().is_some());
    }

    #[test]
    fn test_cli_has_about() {
        let cmd = Cli::command();
        assert!(cmd.get_about().is_some());
    }

    #[test]
    fn test_cli_name() {
        let cmd = Cli::command();
        assert_eq!(cmd.get_name(), "edgelord");
    }

    // Tests for ColorChoice enum

    #[test]
    fn test_color_choice_default_is_auto() {
        let choice = ColorChoice::default();
        assert!(matches!(choice, ColorChoice::Auto));
    }

    #[test]
    fn test_color_choice_clone() {
        let choice = ColorChoice::Always;
        let cloned = choice.clone();
        assert!(matches!(cloned, ColorChoice::Always));
    }

    #[test]
    fn test_color_choice_debug() {
        let choice = ColorChoice::Never;
        let debug_str = format!("{:?}", choice);
        assert!(debug_str.contains("Never"));
    }

    // Tests for parsing basic CLI options

    #[test]
    fn test_parse_run_command() {
        let cli = Cli::try_parse_from(["edgelord", "run"]).unwrap();
        assert!(matches!(cli.command, Commands::Run(_)));
        assert!(!cli.json);
        assert!(!cli.quiet);
        assert_eq!(cli.verbose, 0);
    }

    #[test]
    fn test_parse_json_flag() {
        let cli = Cli::try_parse_from(["edgelord", "--json", "run"]).unwrap();
        assert!(cli.json);
    }

    #[test]
    fn test_parse_quiet_flag() {
        let cli = Cli::try_parse_from(["edgelord", "--quiet", "run"]).unwrap();
        assert!(cli.quiet);
    }

    #[test]
    fn test_parse_short_quiet_flag() {
        let cli = Cli::try_parse_from(["edgelord", "-q", "run"]).unwrap();
        assert!(cli.quiet);
    }

    #[test]
    fn test_parse_verbose_single() {
        let cli = Cli::try_parse_from(["edgelord", "-v", "run"]).unwrap();
        assert_eq!(cli.verbose, 1);
    }

    #[test]
    fn test_parse_verbose_double() {
        let cli = Cli::try_parse_from(["edgelord", "-vv", "run"]).unwrap();
        assert_eq!(cli.verbose, 2);
    }

    #[test]
    fn test_parse_verbose_triple() {
        let cli = Cli::try_parse_from(["edgelord", "-vvv", "run"]).unwrap();
        assert_eq!(cli.verbose, 3);
    }

    #[test]
    fn test_parse_verbose_long_flag() {
        let cli = Cli::try_parse_from(["edgelord", "--verbose", "--verbose", "run"]).unwrap();
        assert_eq!(cli.verbose, 2);
    }

    #[test]
    fn test_parse_color_auto() {
        let cli = Cli::try_parse_from(["edgelord", "--color", "auto", "run"]).unwrap();
        assert!(matches!(cli.color, ColorChoice::Auto));
    }

    #[test]
    fn test_parse_color_always() {
        let cli = Cli::try_parse_from(["edgelord", "--color", "always", "run"]).unwrap();
        assert!(matches!(cli.color, ColorChoice::Always));
    }

    #[test]
    fn test_parse_color_never() {
        let cli = Cli::try_parse_from(["edgelord", "--color", "never", "run"]).unwrap();
        assert!(matches!(cli.color, ColorChoice::Never));
    }

    // Tests for RunArgs parsing

    #[test]
    fn test_run_args_defaults() {
        let cli = Cli::try_parse_from(["edgelord", "run"]).unwrap();
        if let Commands::Run(args) = cli.command {
            assert!(!args.dry_run);
            assert!(!args.json_logs);
            assert!(!args.telegram_enabled);
            assert!(!args.mainnet);
            assert!(!args.testnet);
            assert!(args.chain_id.is_none());
            assert!(args.log_level.is_none());
            assert!(args.strategies.is_none());
            assert!(args.min_edge.is_none());
            assert!(args.min_profit.is_none());
            assert!(args.max_exposure.is_none());
            assert!(args.max_position.is_none());
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_run_args_dry_run() {
        let cli = Cli::try_parse_from(["edgelord", "run", "--dry-run"]).unwrap();
        if let Commands::Run(args) = cli.command {
            assert!(args.dry_run);
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_run_args_chain_id() {
        let cli = Cli::try_parse_from(["edgelord", "run", "--chain-id", "137"]).unwrap();
        if let Commands::Run(args) = cli.command {
            assert_eq!(args.chain_id, Some(137));
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_run_args_mainnet_flag() {
        let cli = Cli::try_parse_from(["edgelord", "run", "--mainnet"]).unwrap();
        if let Commands::Run(args) = cli.command {
            assert!(args.mainnet);
            assert!(!args.testnet);
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_run_args_testnet_flag() {
        let cli = Cli::try_parse_from(["edgelord", "run", "--testnet"]).unwrap();
        if let Commands::Run(args) = cli.command {
            assert!(args.testnet);
            assert!(!args.mainnet);
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_run_args_mainnet_testnet_conflict() {
        // mainnet and testnet are mutually exclusive
        let result = Cli::try_parse_from(["edgelord", "run", "--mainnet", "--testnet"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_run_args_strategies() {
        let cli = Cli::try_parse_from(["edgelord", "run", "--strategies", "binary,multi"]).unwrap();
        if let Commands::Run(args) = cli.command {
            assert_eq!(args.strategies, Some("binary,multi".to_string()));
        } else {
            panic!("Expected Run command");
        }
    }

    // Tests for Statistics subcommands

    #[test]
    fn test_statistics_today_command() {
        let cli = Cli::try_parse_from(["edgelord", "statistics", "today"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Statistics(StatsCommand::Today(_))
        ));
    }

    #[test]
    fn test_statistics_week_command() {
        let cli = Cli::try_parse_from(["edgelord", "statistics", "week"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Statistics(StatsCommand::Week(_))
        ));
    }

    #[test]
    fn test_statistics_history_command() {
        let cli = Cli::try_parse_from(["edgelord", "statistics", "history"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Statistics(StatsCommand::History(_))
        ));
    }

    #[test]
    fn test_statistics_history_with_days() {
        let cli = Cli::try_parse_from(["edgelord", "statistics", "history", "60"]).unwrap();
        if let Commands::Statistics(StatsCommand::History(args)) = cli.command {
            assert_eq!(args.days, 60);
        } else {
            panic!("Expected History command");
        }
    }

    #[test]
    fn test_statistics_history_default_days() {
        let cli = Cli::try_parse_from(["edgelord", "statistics", "history"]).unwrap();
        if let Commands::Statistics(StatsCommand::History(args)) = cli.command {
            assert_eq!(args.days, 30);
        } else {
            panic!("Expected History command");
        }
    }

    #[test]
    fn test_statistics_export_command() {
        let cli = Cli::try_parse_from(["edgelord", "statistics", "export"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Statistics(StatsCommand::Export(_))
        ));
    }

    #[test]
    fn test_statistics_export_with_output() {
        let cli =
            Cli::try_parse_from(["edgelord", "statistics", "export", "-o", "stats.csv"]).unwrap();
        if let Commands::Statistics(StatsCommand::Export(args)) = cli.command {
            assert_eq!(args.output, Some(PathBuf::from("stats.csv")));
        } else {
            panic!("Expected Export command");
        }
    }

    #[test]
    fn test_statistics_prune_command() {
        let cli = Cli::try_parse_from(["edgelord", "statistics", "prune"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Statistics(StatsCommand::Prune(_))
        ));
    }

    #[test]
    fn test_statistics_prune_default_days() {
        let cli = Cli::try_parse_from(["edgelord", "statistics", "prune"]).unwrap();
        if let Commands::Statistics(StatsCommand::Prune(args)) = cli.command {
            assert_eq!(args.days, 30);
        } else {
            panic!("Expected Prune command");
        }
    }

    // Tests for Config subcommands

    #[test]
    fn test_config_init_command() {
        let cli = Cli::try_parse_from(["edgelord", "config", "init"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Config(ConfigCommand::Init(_))
        ));
    }

    #[test]
    fn test_config_init_with_force() {
        let cli = Cli::try_parse_from(["edgelord", "config", "init", "--force"]).unwrap();
        if let Commands::Config(ConfigCommand::Init(args)) = cli.command {
            assert!(args.force);
        } else {
            panic!("Expected Config Init command");
        }
    }

    #[test]
    fn test_config_show_command() {
        let cli = Cli::try_parse_from(["edgelord", "config", "show"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Config(ConfigCommand::Show(_))
        ));
    }

    #[test]
    fn test_config_validate_command() {
        let cli = Cli::try_parse_from(["edgelord", "config", "validate"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Config(ConfigCommand::Validate(_))
        ));
    }

    // Tests for Check subcommands

    #[test]
    fn test_check_config_command() {
        let cli = Cli::try_parse_from(["edgelord", "check", "config"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Check(CheckCommand::Config(_))
        ));
    }

    #[test]
    fn test_check_live_command() {
        let cli = Cli::try_parse_from(["edgelord", "check", "live"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Check(CheckCommand::Live(_))
        ));
    }

    #[test]
    fn test_check_health_command() {
        let cli = Cli::try_parse_from(["edgelord", "check", "health"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Check(CheckCommand::Health(_))
        ));
    }

    #[test]
    fn test_check_connection_command() {
        let cli = Cli::try_parse_from(["edgelord", "check", "connection"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Check(CheckCommand::Connection(_))
        ));
    }

    #[test]
    fn test_check_telegram_command() {
        let cli = Cli::try_parse_from(["edgelord", "check", "telegram"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Check(CheckCommand::Telegram(_))
        ));
    }

    // Tests for Wallet subcommands

    #[test]
    fn test_wallet_approve_command() {
        let cli = Cli::try_parse_from(["edgelord", "wallet", "approve"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Wallet(WalletCommand::Approve(_))
        ));
    }

    #[test]
    fn test_wallet_approve_with_amount() {
        let cli =
            Cli::try_parse_from(["edgelord", "wallet", "approve", "--amount", "5000"]).unwrap();
        if let Commands::Wallet(WalletCommand::Approve(args)) = cli.command {
            assert_eq!(args.amount, Decimal::from(5000));
        } else {
            panic!("Expected Wallet Approve command");
        }
    }

    #[test]
    fn test_wallet_approve_default_amount() {
        let cli = Cli::try_parse_from(["edgelord", "wallet", "approve"]).unwrap();
        if let Commands::Wallet(WalletCommand::Approve(args)) = cli.command {
            assert_eq!(args.amount, Decimal::from(10000));
        } else {
            panic!("Expected Wallet Approve command");
        }
    }

    #[test]
    fn test_wallet_approve_with_yes() {
        let cli = Cli::try_parse_from(["edgelord", "wallet", "approve", "--yes"]).unwrap();
        if let Commands::Wallet(WalletCommand::Approve(args)) = cli.command {
            assert!(args.yes);
        } else {
            panic!("Expected Wallet Approve command");
        }
    }

    #[test]
    fn test_wallet_status_command() {
        let cli = Cli::try_parse_from(["edgelord", "wallet", "status"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Wallet(WalletCommand::Status(_))
        ));
    }

    #[test]
    fn test_wallet_address_command() {
        let cli = Cli::try_parse_from(["edgelord", "wallet", "address"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Wallet(WalletCommand::Address(_))
        ));
    }

    #[test]
    fn test_wallet_sweep_command() {
        let cli = Cli::try_parse_from([
            "edgelord",
            "wallet",
            "sweep",
            "--to",
            "0x1234567890123456789012345678901234567890",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Commands::Wallet(WalletCommand::Sweep(_))
        ));
    }

    #[test]
    fn test_wallet_sweep_defaults() {
        let cli = Cli::try_parse_from([
            "edgelord",
            "wallet",
            "sweep",
            "--to",
            "0x1234567890123456789012345678901234567890",
        ])
        .unwrap();
        if let Commands::Wallet(WalletCommand::Sweep(args)) = cli.command {
            assert_eq!(args.asset, "usdc");
            assert_eq!(args.network, "polygon");
            assert!(!args.yes);
        } else {
            panic!("Expected Wallet Sweep command");
        }
    }

    #[test]
    fn test_wallet_sweep_requires_to() {
        let result = Cli::try_parse_from(["edgelord", "wallet", "sweep"]);
        assert!(result.is_err());
    }

    // Tests for Strategy subcommands

    #[test]
    fn test_strategies_list_command() {
        let cli = Cli::try_parse_from(["edgelord", "strategies", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Strategies(StrategyCommand::List)
        ));
    }

    #[test]
    fn test_strategies_explain_command() {
        let cli = Cli::try_parse_from(["edgelord", "strategies", "explain", "binary"]).unwrap();
        if let Commands::Strategies(StrategyCommand::Explain { name }) = cli.command {
            assert_eq!(name, "binary");
        } else {
            panic!("Expected Strategies Explain command");
        }
    }

    #[test]
    fn test_strategies_explain_requires_name() {
        let result = Cli::try_parse_from(["edgelord", "strategies", "explain"]);
        assert!(result.is_err());
    }

    // Tests for other commands

    #[test]
    fn test_status_command() {
        let cli = Cli::try_parse_from(["edgelord", "status"]).unwrap();
        assert!(matches!(cli.command, Commands::Status(_)));
    }

    #[test]
    fn test_init_command() {
        let cli = Cli::try_parse_from(["edgelord", "init"]).unwrap();
        assert!(matches!(cli.command, Commands::Init(_)));
    }

    #[test]
    fn test_init_with_force() {
        let cli = Cli::try_parse_from(["edgelord", "init", "--force"]).unwrap();
        if let Commands::Init(args) = cli.command {
            assert!(args.force);
        } else {
            panic!("Expected Init command");
        }
    }

    // Tests for error cases

    #[test]
    fn test_unknown_command_fails() {
        let result = Cli::try_parse_from(["edgelord", "unknown"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_color_value() {
        let result = Cli::try_parse_from(["edgelord", "--color", "invalid", "run"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_chain_id_type() {
        let result = Cli::try_parse_from(["edgelord", "run", "--chain-id", "not_a_number"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_subcommand() {
        let result = Cli::try_parse_from(["edgelord"]);
        assert!(result.is_err());
    }

    // Tests for global flag placement

    #[test]
    fn test_global_flags_before_command() {
        let cli = Cli::try_parse_from(["edgelord", "--json", "--quiet", "-vv", "run"]).unwrap();
        assert!(cli.json);
        assert!(cli.quiet);
        assert_eq!(cli.verbose, 2);
    }

    #[test]
    fn test_global_flags_after_command() {
        let cli = Cli::try_parse_from(["edgelord", "run", "--json", "--quiet", "-vv"]).unwrap();
        assert!(cli.json);
        assert!(cli.quiet);
        assert_eq!(cli.verbose, 2);
    }

    #[test]
    fn test_global_flags_mixed_position() {
        let cli = Cli::try_parse_from(["edgelord", "--json", "run", "--dry-run", "-v"]).unwrap();
        assert!(cli.json);
        assert_eq!(cli.verbose, 1);
        if let Commands::Run(args) = cli.command {
            assert!(args.dry_run);
        } else {
            panic!("Expected Run command");
        }
    }
}
