use clap::Parser;
use edgelord::cli::{
    CheckCommand, Cli, Commands, ConfigCommand, ServiceCommand, StatisticsCommand, WalletCommand,
};

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Run(args) => edgelord::cli::run::execute(&args).await,
        Commands::Status(args) => {
            edgelord::cli::status::execute(&args.db);
            Ok(())
        }
        Commands::Statistics(cmd) => match cmd {
            StatisticsCommand::Today(args) => edgelord::cli::statistics::execute_today(&args.db),
            StatisticsCommand::Week(args) => edgelord::cli::statistics::execute_week(&args.db),
            StatisticsCommand::History(args) => {
                edgelord::cli::statistics::execute_history(&args.db, args.days)
            }
            StatisticsCommand::Export(args) => edgelord::cli::statistics::execute_export(
                &args.db,
                args.days,
                args.output.as_deref(),
            ),
            StatisticsCommand::Prune(args) => {
                edgelord::cli::statistics::execute_prune(&args.db, args.days)
            }
        },
        Commands::Config(cmd) => match cmd {
            ConfigCommand::Init(args) => {
                edgelord::cli::config::execute_init(&args.path, args.force)
            }
            ConfigCommand::Show(args) => edgelord::cli::config::execute_show(&args.config),
            ConfigCommand::Validate(args) => edgelord::cli::config::execute_validate(&args.config),
        },
        Commands::Logs(args) => {
            edgelord::cli::logs::execute(&args);
            Ok(())
        }
        Commands::Provision(cmd) => edgelord::cli::provision::execute(cmd).await,
        Commands::Service(cmd) => match cmd {
            ServiceCommand::Install(args) => edgelord::cli::service::execute_install(&args),
            ServiceCommand::Uninstall => edgelord::cli::service::execute_uninstall(),
        },
        Commands::Check(cmd) => match cmd {
            CheckCommand::Config(args) => edgelord::cli::check::execute_config(&args.config),
            CheckCommand::Live(args) => edgelord::cli::check::execute_live(&args.config),
            CheckCommand::Connection(args) => {
                edgelord::cli::check::execute_connection(&args.config).await
            }
            CheckCommand::Telegram(args) => {
                edgelord::cli::check::execute_telegram(&args.config).await
            }
        },
        Commands::Wallet(cmd) => match cmd {
            WalletCommand::Approve(args) => {
                edgelord::cli::wallet::execute_approve(&args.config, args.amount, args.yes).await
            }
            WalletCommand::Status(args) => {
                edgelord::cli::wallet::execute_status(&args.config).await
            }
            WalletCommand::Address(args) => edgelord::cli::wallet::execute_address(&args.config),
            WalletCommand::Sweep(args) => {
                edgelord::cli::wallet::execute_sweep(
                    &args.config,
                    &args.to,
                    &args.asset,
                    &args.network,
                    args.yes,
                )
                .await
            }
        },
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
