use clap::Parser;
use edgelord::adapter::inbound::cli::{
    self,
    command::{
        CheckCommand, Cli, ColorChoice, Commands, ConfigCommand, StatsCommand, StrategyCommand,
        WalletCommand,
    },
    output,
};
use edgelord::infrastructure::operator::entry::Operator;

fn setup_colors(choice: ColorChoice) {
    match choice {
        ColorChoice::Auto => {
            // owo-colors auto-detects TTY by default
        }
        ColorChoice::Always => {
            owo_colors::set_override(true);
        }
        ColorChoice::Never => {
            owo_colors::set_override(false);
        }
    }
}

#[tokio::main]
async fn main() {
    // Install rustls crypto provider before any TLS operations
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let _ = dotenvy::dotenv();
    let _ = cli::operator::install(Box::new(Operator));

    let cli = Cli::parse();
    output::configure(output::OutputConfig::new(cli.json, cli.quiet, cli.verbose));
    if cli.json {
        setup_colors(ColorChoice::Never);
    } else {
        setup_colors(cli.color.clone());
    }

    let result = match cli.command {
        Commands::Run(args) => cli::run::execute(&args).await,
        Commands::Status(args) => {
            cli::status::execute(&args.db, args.config.as_deref());
            Ok(())
        }
        Commands::Statistics(cmd) => match cmd {
            StatsCommand::Today(args) => cli::stats::handler::execute_today(&args.db),
            StatsCommand::Week(args) => cli::stats::handler::execute_week(&args.db),
            StatsCommand::History(args) => {
                cli::stats::handler::execute_history(&args.db, args.days)
            }
            StatsCommand::Export(args) => {
                cli::stats::handler::execute_export(&args.db, args.days, args.output.as_deref())
            }
            StatsCommand::Prune(args) => cli::stats::handler::execute_prune(&args.db, args.days),
        },
        Commands::Config(cmd) => match cmd {
            ConfigCommand::Init(args) => cli::config::execute_init(&args.path, args.force),
            ConfigCommand::Show(args) => cli::config::execute_show(&args.config),
            ConfigCommand::Validate(args) => cli::config::execute_validate(&args.config),
        },
        Commands::Provision(cmd) => cli::provision::command::execute(cmd).await,
        Commands::Check(cmd) => match cmd {
            CheckCommand::Config(args) => cli::check::config::execute_config(&args.config),
            CheckCommand::Live(args) => cli::check::live::execute_live(&args.config),
            CheckCommand::Health(args) => cli::check::health::execute_health(&args.config),
            CheckCommand::Connection(args) => {
                cli::check::connection::execute_connection(&args.config).await
            }
            CheckCommand::Telegram(args) => {
                cli::check::telegram::execute_telegram(&args.config).await
            }
        },
        Commands::Wallet(cmd) => match cmd {
            WalletCommand::Approve(args) => {
                cli::wallet::approve::execute_approve(&args.config, args.amount, args.yes).await
            }
            WalletCommand::Status(args) => cli::wallet::status::execute_status(&args.config).await,
            WalletCommand::Address(args) => cli::wallet::address::execute_address(&args.config),
            WalletCommand::Sweep(args) => {
                cli::wallet::sweep::execute_sweep(
                    &args.config,
                    &args.to,
                    &args.asset,
                    &args.network,
                    args.yes,
                )
                .await
            }
        },
        Commands::Init(args) => cli::init::execute(args.path, args.force),
        Commands::Strategies(cmd) => match cmd {
            StrategyCommand::List => cli::strategy::list(),
            StrategyCommand::Explain { name } => cli::strategy::explain(&name),
        },
    };

    if let Err(e) = result {
        render_error(&e);
        std::process::exit(1);
    }
}

/// Render an error with helpful context (Astral-style).
fn render_error(error: &edgelord::error::Error) {
    use edgelord::error::{ConfigError, Error, ExecutionError, RiskError};

    // Primary error message
    output::error(&error.to_string());

    // Add contextual hints for common error types
    match error {
        Error::Config(ConfigError::ReadFile(_)) => {
            output::hint("check that the config file exists and is readable");
        }
        Error::Config(ConfigError::Parse(_)) => {
            output::hint("run `edgelord config validate` to check your configuration");
        }
        Error::Config(ConfigError::MissingField { field }) => {
            output::hint(&format!("add the `{field}` field to your config file"));
        }
        Error::Connection(_) => {
            output::hint("check your network connection and exchange status");
        }
        Error::Execution(ExecutionError::AuthFailed(_)) => {
            output::hint("verify your API credentials in the config file");
        }
        Error::Risk(RiskError::CircuitBreakerActive { .. }) => {
            output::hint("use `edgelord status` to check circuit breaker state");
        }
        Error::Risk(RiskError::ExposureLimitExceeded { .. }) => {
            output::hint("adjust max_total_exposure in your risk config or close some positions");
        }
        Error::Database(_) => {
            output::hint("check that the database path is writable");
        }
        _ => {}
    }
}
